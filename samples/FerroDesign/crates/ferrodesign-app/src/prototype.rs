//! Native FerroUI prototype host. Presentation uses a disposable preview of the
//! authored document, activation-scoped Dispatcher timers and explicit external
//! requests. This module adds no rendering backend and no browser-only clock.

use crate::canvas::{DesignCanvas, Tool};
use ferrodesign_core::{
    Document, DocumentError, Editor, NodeId, PrototypeAction, PrototypePlayer, PrototypeTimer,
    Trigger, Viewport, MAX_DOCUMENT_DEPTH,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::Ref;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq)]
pub enum PrototypeRuntimeEvent {
    Started(NodeId),
    Stopped,
    Changed,
    /// A validated request awaiting a host implementation, not a completed
    /// browser launch. Use take_pending_url after accepting/handling it.
    OpenUrl(String),
    /// The renderer has no scroll-container offset yet. Keep this request
    /// explicit instead of replacing content scrolling with camera panning.
    ScrollTo(NodeId),
}

struct AuthoredSnapshot {
    document: Document,
    selection: Vec<NodeId>,
    active_page: NodeId,
    viewport: Viewport,
    tool: Tool,
}

struct ScheduledTimer {
    subscription: Rc<dyn IDisposable>,
}

pub struct PrototypeRuntime {
    editor: Rc<RefCell<Editor>>,
    canvas: Ref<DesignCanvas>,
    player: RefCell<PrototypePlayer>,
    authored: RefCell<Option<AuthoredSnapshot>>,
    timers: RefCell<BTreeMap<PrototypeTimer, ScheduledTimer>>,
    generation: Cell<u64>,
    callback: RefCell<Option<Rc<dyn Fn(PrototypeRuntimeEvent)>>>,
}

impl PrototypeRuntime {
    pub fn new(editor: Rc<RefCell<Editor>>, canvas: Ref<DesignCanvas>) -> Rc<Self> {
        Rc::new(Self {
            editor,
            canvas,
            player: RefCell::new(PrototypePlayer::default()),
            authored: RefCell::new(None),
            timers: RefCell::new(BTreeMap::new()),
            generation: Cell::new(0),
            callback: RefCell::new(None),
        })
    }

    pub fn set_callback(&self, callback: impl Fn(PrototypeRuntimeEvent) + 'static) {
        *self.callback.borrow_mut() = Some(Rc::new(callback));
    }

    pub fn is_active(&self) -> bool {
        self.authored.borrow().is_some()
    }

    pub fn current(&self) -> Option<NodeId> {
        self.player.borrow().current
    }

    /// Exports and autosaves always read authored state, including during a
    /// preview that changed bound fills, text, visibility or variable overrides.
    pub fn document_json(&self) -> Result<String, DocumentError> {
        if let Some(authored) = self.authored.borrow().as_ref() {
            authored.document.to_json()
        } else {
            self.editor.borrow().to_json()
        }
    }

    pub fn start(self: &Rc<Self>, root: NodeId) -> bool {
        self.canvas.finish_pen();
        self.canvas.cancel_active_edit();
        self.canvas.reset_presentation_input();
        let document = self
            .authored
            .borrow()
            .as_ref()
            .map(|authored| authored.document.clone())
            .unwrap_or_else(|| self.editor.borrow().document.clone());
        if !self.player.borrow_mut().start_at(&document, root) {
            return false;
        }
        self.cancel_timers();
        self.generation.set(self.generation.get().wrapping_add(1));
        {
            let mut editor = self.editor.borrow_mut();
            if self.authored.borrow().is_none() {
                // Finish an existing pointer transaction before capturing its
                // final authored state; preview actions never enter history.
                editor.commit_transaction();
                *self.authored.borrow_mut() = Some(AuthoredSnapshot {
                    document: editor.document.clone(),
                    selection: editor.selection.clone(),
                    active_page: editor.active_page,
                    viewport: self.canvas.viewport.get(),
                    tool: self.canvas.tool.get(),
                });
            }
            editor.document = document;
            if let Some(page) = editor.document.page_of(root) {
                editor.active_page = page;
            }
            editor.refresh();
        }
        self.canvas.leave_vector_edit();
        self.canvas.presentation.set(Some(root));
        self.canvas.overlays.borrow_mut().clear();
        self.canvas.reset_fit();
        self.canvas.focus();
        self.synchronize_timers();
        self.emit(PrototypeRuntimeEvent::Started(root));
        true
    }

    pub fn stop(&self) -> bool {
        let Some(authored) = self.authored.borrow_mut().take() else {
            return false;
        };
        self.generation.set(self.generation.get().wrapping_add(1));
        self.canvas.reset_presentation_input();
        self.cancel_timers();
        self.player.borrow_mut().stop();
        {
            let mut editor = self.editor.borrow_mut();
            editor.document = authored.document;
            editor.selection = authored.selection;
            editor.active_page = authored.active_page;
            editor.refresh();
        }
        self.canvas.presentation.set(None);
        self.canvas.overlays.borrow_mut().clear();
        self.canvas.viewport.set(authored.viewport);
        self.canvas.tool.set(authored.tool);
        self.canvas.invalidate_visual();
        self.emit(PrototypeRuntimeEvent::Stopped);
        true
    }

    pub fn back(self: &Rc<Self>) -> bool {
        if !self.is_active() {
            return false;
        }
        let previous = self.current();
        let changed = self
            .player
            .borrow_mut()
            .back(&self.editor.borrow().document);
        if changed {
            self.after_action(previous);
        }
        changed
    }

    /// Bubble pointer events through the active root's ancestry. The topmost
    /// overlay is modal; locked design layers still respond in presentation.
    pub fn dispatch(self: &Rc<Self>, node: NodeId, trigger: Trigger) -> bool {
        if !self.is_active() || matches!(trigger, Trigger::Key | Trigger::AfterDelay) {
            return false;
        }
        let previous = self.current();
        let changed = {
            let mut editor = self.editor.borrow_mut();
            let mut player = self.player.borrow_mut();
            let root = player.interaction_root();
            let mut source = Some(node);
            let mut changed = false;
            for _ in 0..MAX_DOCUMENT_DEPTH {
                let Some(id) = source else {
                    break;
                };
                if player.trigger(&mut editor.document, id, trigger) {
                    changed = true;
                    break;
                }
                if Some(id) == root {
                    break;
                }
                source = editor.document.nodes.get(&id).and_then(|node| node.parent);
            }
            changed
        };
        if changed {
            self.after_action(previous);
        }
        changed
    }

    /// Resolve key chords from topmost descendants toward the presentation
    /// root. Only a matching authored key rule can consume the key event.
    pub fn key(self: &Rc<Self>, key: &str) -> bool {
        if !self.is_active() {
            return false;
        }
        let previous = self.current();
        let changed = {
            let mut editor = self.editor.borrow_mut();
            let mut player = self.player.borrow_mut();
            let Some(root) = player.interaction_root() else {
                return false;
            };
            editor
                .document
                .descendants(root)
                .into_iter()
                .rev()
                .any(|node| player.trigger_key(&mut editor.document, node, key))
        };
        if changed {
            self.after_action(previous);
        }
        changed
    }

    pub fn take_pending_url(&self) -> Option<String> {
        self.player.borrow_mut().pending_url.take()
    }

    pub fn take_scroll_target(&self) -> Option<NodeId> {
        self.player.borrow_mut().scroll_target.take()
    }

    fn after_action(self: &Rc<Self>, previous: Option<NodeId>) {
        let (current, overlays, action, url, scroll) = {
            let player = self.player.borrow();
            (
                player.current,
                player.overlays.clone(),
                player.last_action.clone(),
                player.pending_url.clone(),
                player.scroll_target,
            )
        };
        {
            let mut editor = self.editor.borrow_mut();
            if let Some(page) = current.and_then(|id| editor.document.page_of(id)) {
                editor.active_page = page;
            }
            editor.refresh();
        }
        if matches!(
            action,
            Some(
                PrototypeAction::NavigateTo { .. }
                    | PrototypeAction::OpenOverlay { .. }
                    | PrototypeAction::CloseOverlay
                    | PrototypeAction::Back
            )
        ) {
            self.canvas.reset_presentation_input();
        }
        self.canvas.presentation.set(current);
        *self.canvas.overlays.borrow_mut() = overlays;
        if current != previous || matches!(action, Some(PrototypeAction::NavigateTo { .. })) {
            self.canvas.reset_fit();
        } else {
            self.canvas.invalidate_visual();
        }
        self.synchronize_timers();
        self.emit(PrototypeRuntimeEvent::Changed);
        if !self.is_active() {
            return;
        }
        match action {
            Some(PrototypeAction::OpenUrl { .. }) => {
                if let Some(url) = url {
                    self.emit(PrototypeRuntimeEvent::OpenUrl(url));
                }
            }
            Some(PrototypeAction::ScrollTo { .. }) => {
                if let Some(target) = scroll {
                    self.emit(PrototypeRuntimeEvent::ScrollTo(target));
                }
            }
            _ => {}
        }
    }

    fn synchronize_timers(self: &Rc<Self>) {
        let wanted = self
            .player
            .borrow_mut()
            .pending_timers(&self.editor.borrow().document);
        let keys: BTreeSet<_> = wanted.iter().copied().collect();
        let obsolete: Vec<_> = self
            .timers
            .borrow()
            .keys()
            .filter(|timer| !keys.contains(timer))
            .copied()
            .collect();
        for timer in obsolete {
            if let Some(timer) = self.timers.borrow_mut().remove(&timer) {
                timer.subscription.dispose();
            }
        }
        for timer in wanted {
            if !self.timers.borrow().contains_key(&timer) {
                self.schedule_timer(timer, Instant::now());
            }
        }
    }

    fn schedule_timer(self: &Rc<Self>, timer: PrototypeTimer, start: Instant) {
        let elapsed = start.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let remaining = (timer.delay_ms as u64).saturating_sub(elapsed);
        // DispatcherTimer periods are signed 32-bit milliseconds. Long delays
        // are split into cancellable chunks without firing the action early.
        // Zero delay always yields at least one UI turn, avoiding recursion.
        let interval = Duration::from_millis(remaining.clamp(1, i32::MAX as u64));
        let generation = self.generation.get();
        let weak = Rc::downgrade(self);
        let subscription = DispatcherTimer::run_once(
            move || {
                let Some(runtime) = weak.upgrade() else {
                    return;
                };
                if runtime.generation.get() != generation || !runtime.is_active() {
                    return;
                }
                runtime.timers.borrow_mut().remove(&timer);
                let elapsed = start.elapsed().as_millis().min(u64::MAX as u128) as u64;
                if elapsed < timer.delay_ms as u64 {
                    // A changed activation can invalidate a long-delay chunk.
                    let pending = runtime
                        .player
                        .borrow_mut()
                        .pending_timers(&runtime.editor.borrow().document);
                    if pending.contains(&timer) {
                        runtime.schedule_timer(timer, start);
                    }
                    return;
                }
                let previous = runtime.current();
                let changed = {
                    let mut editor = runtime.editor.borrow_mut();
                    runtime
                        .player
                        .borrow_mut()
                        .fire_timer(&mut editor.document, timer, elapsed)
                };
                if changed {
                    runtime.after_action(previous);
                } else {
                    runtime.synchronize_timers();
                }
            },
            interval,
            DispatcherPriority::BACKGROUND,
        );
        self.timers
            .borrow_mut()
            .insert(timer, ScheduledTimer { subscription });
    }

    fn cancel_timers(&self) {
        let timers = std::mem::take(&mut *self.timers.borrow_mut());
        for (_, timer) in timers {
            timer.subscription.dispose();
        }
    }

    fn emit(&self, event: PrototypeRuntimeEvent) {
        let callback = self.callback.borrow().clone();
        if let Some(callback) = callback {
            callback(event);
        }
    }
}

impl Drop for PrototypeRuntime {
    fn drop(&mut self) {
        for (_, timer) in std::mem::take(self.timers.get_mut()) {
            timer.subscription.dispose();
        }
        if let Some(authored) = self.authored.get_mut().take() {
            if let Ok(mut editor) = self.editor.try_borrow_mut() {
                editor.document = authored.document;
                editor.selection = authored.selection;
                editor.active_page = authored.active_page;
                editor.refresh();
            }
        }
    }
}
