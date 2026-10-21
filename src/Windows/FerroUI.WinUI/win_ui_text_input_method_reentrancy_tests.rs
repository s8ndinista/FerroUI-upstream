// FerroUI-specific regressions for native-client callbacks. These exercise the
// adapter itself without creating a CoreText context or a desktop window: each
// callback replaces the client before native context creation can be reached.
#[cfg(test)]
mod native_reentrancy_tests {
    use super::*;
    use ferroui_base::input::InputElement;
    use ferroui_base::input::text_input::TextInputMethodClientEvents;
    use ferroui_base::layout::{ILayoutManager, ILayoutRoot, LayoutManager, Layoutable};
    use ferroui_base::reactive::Disposable;
    use ferroui_base::rendering::{
        IHitTester, IPresentationSource, IRenderer, ManagedHitTester, RendererDiagnostics,
        SceneInvalidatedEventArgs,
    };
    use ferroui_base::{Ref, Visual};

    type Callback = RefCell<Option<Box<dyn FnOnce()>>>;

    #[derive(Default)]
    struct ReentrantClient {
        events: TextInputMethodClientEvents,
        connection_callback: Callback,
        text_callback: Callback,
        subscribe_callback: Callback,
        owner_callback: Callback,
        visual_callback: Callback,
        revision_callback: Callback,
        owner: RefCell<Option<Ref<InputElement>>>,
        visual: RefCell<Option<Ref<Visual>>>,
        revision: Cell<Option<u64>>,
        live_subscriptions: Rc<Cell<usize>>,
    }

    fn invoke(callback: &Callback) {
        let callback = callback.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
    }

    impl ReentrantClient {
        fn subscription(&self) -> Rc<dyn IDisposable> {
            let count = self.live_subscriptions.clone();
            count.set(count.get() + 1);
            Disposable::create(move || count.set(count.get() - 1))
        }
    }

    impl TextInputMethodClient for ReentrantClient {
        fn events(&self) -> &TextInputMethodClientEvents {
            &self.events
        }
        fn input_method_connection(&self) -> Option<Rc<TextInputMethodConnection>> {
            invoke(&self.connection_callback);
            None
        }
        fn input_method_owner(&self) -> Option<Ref<InputElement>> {
            invoke(&self.owner_callback);
            self.owner.borrow().clone()
        }
        fn input_method_state_revision(&self) -> Option<u64> {
            invoke(&self.revision_callback);
            self.revision.get()
        }
        fn text_view_visual(&self) -> Option<Ref<Visual>> {
            invoke(&self.visual_callback);
            self.visual.borrow().clone()
        }
        fn supports_preedit(&self) -> bool {
            true
        }
        fn supports_surrounding_text(&self) -> bool {
            true
        }
        fn surrounding_text(&self) -> String {
            invoke(&self.text_callback);
            "stale text".into()
        }
        fn cursor_rectangle(&self) -> FerroRect {
            FerroRect::default()
        }
        fn selection(&self) -> TextSelection {
            TextSelection::default()
        }
        fn set_selection(&self, _: TextSelection) {}
        fn surrounding_text_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
            let subscription = self.subscription();
            invoke(&self.subscribe_callback);
            subscription
        }
        fn selection_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
            self.subscription()
        }
    }

    fn method() -> Rc<WinUITextInputMethod> {
        WinUITextInputMethod::new(
            Weak::new(),
            Box::new(|| None),
            Box::new(|| None),
            Box::new(|| None),
        )
    }

    fn remove_client(method: &Rc<WinUITextInputMethod>) -> Box<dyn FnOnce()> {
        let method = Rc::downgrade(method);
        Box::new(move || method.upgrade().unwrap().set_client(None))
    }

    fn assert_removed(method: &WinUITextInputMethod) {
        assert!(method.client.borrow().is_none());
        assert!(method.connection.borrow().is_none());
        assert!(method.client_events.borrow().is_empty());
        assert!(method.ime_text.borrow().is_empty());
        assert_eq!(method.synchronization.borrow().current().text, "");
        assert!(!method.exact_client.get());
        assert!(!method.is_composing.get());
    }

    #[test]
    fn subscription_disposal_can_replace_client_without_a_live_adapter_borrow() {
        let method = method();
        let client = Rc::new(ReentrantClient::default());
        method
            .client_events
            .borrow_mut()
            .push(Disposable::create(remove_client(&method)));
        method.set_client(Some(client.clone()));
        assert_removed(&method);
        assert_eq!(client.live_subscriptions.get(), 0);
    }

    #[test]
    fn connection_accessor_cannot_overwrite_a_reentrant_client_replacement() {
        let method = method();
        let client = Rc::new(ReentrantClient::default());
        client
            .connection_callback
            .replace(Some(remove_client(&method)));
        method.set_client(Some(client.clone()));
        assert_removed(&method);
        assert_eq!(client.live_subscriptions.get(), 0);
    }

    #[test]
    fn event_registration_disposes_its_subscription_after_reentrant_replacement() {
        let method = method();
        let client = Rc::new(ReentrantClient::default());
        client
            .subscribe_callback
            .replace(Some(remove_client(&method)));
        method.set_client(Some(client.clone()));
        assert_removed(&method);
        assert_eq!(client.live_subscriptions.get(), 0);
    }

    #[test]
    fn reentrant_state_query_cannot_publish_the_previous_clients_text() {
        let method = method();
        let client = Rc::new(ReentrantClient::default());
        client.text_callback.replace(Some(remove_client(&method)));
        method.client.replace(Some(client));
        method.sync_from_client(false);
        assert_removed(&method);
    }

    #[test]
    fn layout_uses_the_semantic_control_before_the_legacy_text_view_fallback() {
        let method = method();
        let client = Rc::new(ReentrantClient::default());
        let owner = InputElement::new();
        let presenter = Visual::new();
        client.owner.replace(Some(owner.clone()));
        client.visual.replace(Some(presenter.clone()));
        method.client.replace(Some(client.clone()));
        let state = method.begin_layout_request().unwrap();
        assert_eq!(method.layout_owner_visual(&state), Some(owner.upcast()));
        client.owner.replace(None);
        let state = method.begin_layout_request().unwrap();
        assert_eq!(method.layout_owner_visual(&state), Some(presenter));
    }

    #[test]
    fn layout_owner_accessor_cannot_fall_back_to_a_replaced_clients_presenter() {
        let method = method();
        let client = Rc::new(ReentrantClient::default());
        method.client.replace(Some(client.clone()));
        let state = method.begin_layout_request().unwrap();
        client.owner_callback.replace(Some(remove_client(&method)));
        client.visual_callback.replace(Some(Box::new(|| {
            panic!("the replaced client's presenter must not be queried")
        })));
        assert!(method.layout_owner_visual(&state).is_none());
        assert_removed(&method);
    }

    #[test]
    fn layout_revision_accessors_cannot_capture_or_validate_a_replacement_owner() {
        let method = method();
        let client = Rc::new(ReentrantClient::default());
        method.client.replace(Some(client.clone()));
        client
            .revision_callback
            .replace(Some(remove_client(&method)));
        assert!(method.begin_layout_request().is_none());
        assert_removed(&method);

        method.client.replace(Some(client.clone()));
        let state = method.begin_layout_request().unwrap();
        client
            .revision_callback
            .replace(Some(remove_client(&method)));
        assert!(!method.layout_request_is_current(&state));
        assert_removed(&method);
    }

    #[test]
    fn layout_rejects_a_deferred_edit_before_public_text_or_selection_events() {
        let method = method();
        let client = Rc::new(ReentrantClient::default());
        client.revision.set(Some(12));
        method.client.replace(Some(client.clone()));
        let state = method.begin_layout_request().unwrap();
        assert!(method.layout_request_is_current(&state));
        client.revision.set(Some(13));
        assert!(!method.layout_request_is_current(&state));
    }

    #[test]
    fn nested_layout_focus_roundtrips_reset_and_geometry_updates_retire_old_requests() {
        let method = method();
        method
            .client
            .replace(Some(Rc::new(ReentrantClient::default())));
        let outer = method.begin_layout_request().unwrap();
        let inner = method.begin_layout_request().unwrap();
        assert!(!method.layout_request_is_current(&outer));
        assert!(method.layout_request_is_current(&inner));

        method.on_panel_focus_changed(true);
        method.on_panel_focus_changed(false);
        assert!(!method.layout_request_is_current(&inner));

        let before_reset = method.begin_layout_request().unwrap();
        method.reset_composition_state();
        assert!(!method.layout_request_is_current(&before_reset));

        let before_geometry = method.begin_layout_request().unwrap();
        // A layout notification can keep the same caret while moving/resizing
        // its whole control; even that equal caret must retire the old request.
        method.set_cursor_rect(method.cursor_rect.get());
        assert!(!method.layout_request_is_current(&before_geometry));
    }

    struct LayoutTestRenderer;

    impl IRenderer for LayoutTestRenderer {
        fn diagnostics(&self) -> Rc<RendererDiagnostics> {
            RendererDiagnostics::new()
        }
        fn scene_invalidated(
            &self,
            _: Rc<dyn Fn(&SceneInvalidatedEventArgs)>,
        ) -> Rc<dyn IDisposable> {
            Disposable::empty()
        }
        fn try_get_render_interface_feature(
            &self,
            _: std::any::TypeId,
        ) -> ferroui_base::rendering::composition::LocalFuture<
            Option<ferroui_base::rendering::composition::RenderInterfaceFeature>,
        > {
            Box::pin(std::future::ready(None))
        }
        fn add_dirty(&self, _: &Visual) {}
        fn recalculate_children(&self, _: &Visual) {}
        fn resized(&self, _: ferroui_base::Size) {}
        fn paint(&self, _: FerroRect) {}
        fn start(&self) {}
        fn stop(&self) {}
        fn dispose(&self) {}
    }

    struct LayoutTestSource {
        this: Weak<Self>,
        root: ferroui_base::WeakRef<InputElement>,
        manager: Rc<LayoutManager>,
    }

    impl LayoutTestSource {
        fn new(root: &Ref<InputElement>) -> Rc<Self> {
            Rc::new_cyclic(|this: &Weak<Self>| {
                let layout: Weak<dyn ILayoutRoot> = this.clone();
                Self {
                    this: this.clone(),
                    root: root.downgrade(),
                    manager: LayoutManager::new(layout),
                }
            })
        }
    }

    impl IInputRoot for LayoutTestSource {
        fn focus_manager(&self) -> Option<Rc<ferroui_base::input::FocusManager>> {
            None
        }
        fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
            None
        }
        fn set_pointer_over_element(&self, _: Option<Ref<InputElement>>) {}
        fn cursor_element(&self) -> Option<Ref<InputElement>> {
            None
        }
        fn set_cursor_element(&self, _: Option<Ref<InputElement>>) {}
        fn root_element(&self) -> Ref<InputElement> {
            self.root.upgrade().unwrap()
        }
        fn focus_root(&self) -> Ref<InputElement> {
            self.root_element()
        }
        fn pointer_over_invalidated(&self) {}
    }

    impl ILayoutRoot for LayoutTestSource {
        fn layout_scaling(&self) -> f64 {
            1.0
        }
        fn layout_manager(&self) -> Rc<dyn ILayoutManager> {
            self.manager.clone()
        }
        fn root_visual(&self) -> Ref<Layoutable> {
            self.root_element().upcast()
        }
    }

    impl IPresentationSource for LayoutTestSource {
        fn root_visual(&self) -> Option<Ref<Visual>> {
            self.root.upgrade().map(Ref::upcast)
        }
        fn render_scaling(&self) -> f64 {
            1.0
        }
        fn renderer(&self) -> Rc<dyn IRenderer> {
            Rc::new(LayoutTestRenderer)
        }
        fn layout_root(&self) -> Rc<dyn ILayoutRoot> {
            self.this.upgrade().unwrap()
        }
        fn client_size(&self) -> ferroui_base::Size {
            self.root_element().bounds().size()
        }
        fn hit_tester(&self) -> Rc<dyn IHitTester> {
            Rc::new(ManagedHitTester::new())
        }
        fn input_root(&self) -> Rc<dyn IInputRoot> {
            self.this.upgrade().unwrap()
        }
    }

    #[test]
    fn layout_final_revision_query_cannot_validate_the_preceding_owner_geometry() {
        let root = InputElement::new();
        let source = LayoutTestSource::new(&root);
        root.set_presentation_source_for_root_visual(Some(source.clone()));
        root.set_bounds(FerroRect::new(0.0, 0.0, 100.0, 40.0));
        let input_root: Rc<dyn IInputRoot> = source;
        let method = WinUITextInputMethod::new(
            Weak::new(),
            Box::new(|| None),
            Box::new(move || Some(input_root.clone())),
            Box::new(|| None),
        );
        let client = Rc::new(ReentrantClient::default());
        client.owner.replace(Some(root.clone()));
        method.client.replace(Some(client.clone()));
        let state = method.begin_layout_request().unwrap();
        let layout = method.capture_layout_owner(&state).unwrap();
        assert_eq!(layout.control_bounds, FerroRect::new(0.0, 0.0, 100.0, 40.0));

        // Arm only the final revision query: the first query validates the
        // client before owner/root lookup. The second must precede geometry
        // comparisons so this resize cannot leave the old rectangle accepted.
        let weak_client = Rc::downgrade(&client);
        let changed = root.clone();
        client.revision_callback.replace(Some(Box::new(move || {
            weak_client
                .upgrade()
                .unwrap()
                .revision_callback
                .replace(Some(Box::new(move || {
                    changed.set_bounds(FerroRect::new(0.0, 0.0, 250.0, 80.0));
                })));
        })));
        assert!(!method.layout_owner_is_current(&state, &layout));
        assert_eq!(root.bounds(), FerroRect::new(0.0, 0.0, 250.0, 80.0));
        assert!(method.layout_request_epoch_is_current(&state));
        root.set_presentation_source_for_root_visual(None);
    }
}
