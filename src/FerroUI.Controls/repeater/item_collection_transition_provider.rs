// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::build_tree_scheduler::queue_render_callback;
use super::item_collection_transition::ItemCollectionTransition;
use super::item_collection_transition_completed_event_args::ItemCollectionTransitionCompletedEventArgs;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObject, FerroObjectImpl, Ref};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

#[derive(Default)]
struct TransitionBatch {
    all: Vec<Rc<ItemCollectionTransition>>,
    animated: Vec<Rc<ItemCollectionTransition>>,
    keepalive: Option<Rc<DispatcherTimer>>,
}

/// Extensible transition provider. Batches begin on the next rendering frame;
/// transitions a subclass does not start still receive completion notification.
#[repr(C)]
pub struct ItemCollectionTransitionProvider {
    base: FerroObject,
    next_batch: Cell<u64>,
    queued_batch: Cell<Option<u64>>,
    batches: RefCell<BTreeMap<u64, TransitionBatch>>,
    completed: HandlerList<dyn Fn(&ItemCollectionTransitionCompletedEventArgs)>,
    animations_enabled: Cell<bool>,
}
ferro_class! {
    ItemCollectionTransitionProvider: FerroObject, virtuals ItemCollectionTransitionProviderImpl: FerroObjectImpl {
        fn start_transitions_core(this, transitions: Vec<Rc<ItemCollectionTransition>>);
        fn should_animate_core(this, transition: Rc<ItemCollectionTransition>) -> bool;
    }
}
ferro_class_info!(ItemCollectionTransitionProvider {
    new: ItemCollectionTransitionProvider::new,
    markup: {
        properties: [
            AnimationsEnabled: bool {
                get: |provider: &Ref<ItemCollectionTransitionProvider>| provider.animations_enabled(),
                set: |provider: &Ref<ItemCollectionTransitionProvider>, enabled: bool| provider.set_animations_enabled(enabled)
            },
        ],
        methods: [
            fn ShouldAnimate(Rc<ItemCollectionTransition>) -> bool =>
                |provider: &Ref<ItemCollectionTransitionProvider>, transition: Rc<ItemCollectionTransition>| provider.should_animate(transition),
            fn StartTransitions(Vec<Rc<ItemCollectionTransition>>) =>
                |provider: &Ref<ItemCollectionTransitionProvider>, transitions: Vec<Rc<ItemCollectionTransition>>| provider.start_transitions(transitions),
            fn QueueTransition(Rc<ItemCollectionTransition>) =>
                |provider: &Ref<ItemCollectionTransitionProvider>, transition: Rc<ItemCollectionTransition>| provider.queue_transition(transition),
        ],
        events: [
            TransitionCompleted(Option<Ref<ItemCollectionTransitionProvider>>, ItemCollectionTransitionCompletedEventArgs) =>
                |provider: &Ref<ItemCollectionTransitionProvider>, handler: ferroui_base::metadata::MarkupDelegate| {
                    let sender = provider.downgrade();
                    provider.transition_completed(move |args| {
                        handler.invoke(&[
                            ferroui_base::metadata::into_markup_value(sender.upgrade()),
                            ferroui_base::metadata::into_markup_value(args.clone()),
                        ]);
                    })
                },
        ],
    },
});
impl FerroObjectImpl for ItemCollectionTransitionProvider {}
impl ItemCollectionTransitionProviderImpl for ItemCollectionTransitionProvider {
    fn start_transitions_core(_this: &Self, _transitions: Vec<Rc<ItemCollectionTransition>>) {
        panic!("A derived transition provider must override StartTransitions.");
    }
    fn should_animate_core(_this: &Self, _transition: Rc<ItemCollectionTransition>) -> bool {
        panic!("A derived transition provider must override ShouldAnimateCore.");
    }
}
impl ItemCollectionTransitionProvider {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            next_batch: Cell::new(0),
            queued_batch: Cell::new(None),
            batches: RefCell::new(BTreeMap::new()),
            completed: HandlerList::new(),
            animations_enabled: Cell::new(true),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Supplies the host's motion preference. Platform settings adapters can
    /// update this without changing a custom provider's animation policy.
    pub fn animations_enabled(&self) -> bool {
        self.animations_enabled.get()
    }
    pub fn set_animations_enabled(&self, enabled: bool) {
        self.animations_enabled.set(enabled);
    }
    pub fn should_animate(&self, transition: Rc<ItemCollectionTransition>) -> bool {
        self.should_animate_core(transition)
    }
    pub fn start_transitions(&self, transitions: Vec<Rc<ItemCollectionTransition>>) {
        self.start_transitions_core(transitions);
    }
    pub fn queue_transition(&self, transition: Rc<ItemCollectionTransition>) {
        let (batch, schedule) = match self.queued_batch.get() {
            Some(batch) => (batch, false),
            None => {
                let batch = self.next_batch.get().wrapping_add(1);
                self.next_batch.set(batch);
                self.queued_batch.set(Some(batch));
                self.batches
                    .borrow_mut()
                    .insert(batch, TransitionBatch::default());
                (batch, true)
            }
        };
        let animate = self.animations_enabled() && self.should_animate(transition.clone());
        {
            let mut batches = self.batches.borrow_mut();
            let batch = batches.entry(batch).or_default();
            batch.all.push(transition.clone());
            if animate {
                batch.animated.push(transition);
            }
        }
        if schedule {
            let weak = self.to_ref().downgrade();
            queue_render_callback(move || {
                if let Some(this) = weak.upgrade() {
                    this.on_rendering(batch);
                }
            });
        }
    }
    pub fn transition_completed(
        &self,
        handler: impl Fn(&ItemCollectionTransitionCompletedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.completed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.completed.remove(token);
            }
        })
    }
    pub fn notify_transition_completed(&self, transition: Rc<ItemCollectionTransition>) {
        let args = ItemCollectionTransitionCompletedEventArgs::new(transition);
        for (_, handler) in self.completed.snapshot().iter() {
            handler(&args);
        }
    }
    pub fn retained_batch_count(&self) -> usize {
        self.batches.borrow().len()
    }
    fn on_rendering(&self, batch_id: u64) {
        if self.queued_batch.get() == Some(batch_id) {
            self.queued_batch.set(None);
        }
        let Some((all, animated)) = self
            .batches
            .borrow()
            .get(&batch_id)
            .map(|batch| (batch.all.clone(), batch.animated.clone()))
        else {
            return;
        };
        if !animated.is_empty() {
            self.start_transitions_core(animated);
        }
        let mut keepalive_required = false;
        for transition in all {
            if transition.has_started() {
                keepalive_required = true;
            } else {
                self.notify_transition_completed(transition);
            }
        }
        if keepalive_required {
            let weak = self.to_ref().downgrade();
            let timer = DispatcherTimer::with_callback(
                Duration::from_secs(5),
                DispatcherPriority::BACKGROUND,
                move |timer| {
                    timer.stop();
                    if let Some(this) = weak.upgrade() {
                        let removed = this.batches.borrow_mut().remove(&batch_id);
                        drop(removed);
                    }
                },
            );
            if let Some(batch) = self.batches.borrow_mut().get_mut(&batch_id) {
                batch.keepalive = Some(timer);
            } else {
                timer.stop();
            }
        } else {
            let removed = self.batches.borrow_mut().remove(&batch_id);
            drop(removed);
        }
    }
}
impl Drop for ItemCollectionTransitionProvider {
    fn drop(&mut self) {
        for batch in self.batches.get_mut().values() {
            if let Some(timer) = &batch.keepalive {
                timer.stop();
            }
        }
    }
}
