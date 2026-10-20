// FerroUI-specific regressions for native-client callbacks. These exercise the
// adapter itself without creating a CoreText context or a desktop window: each
// callback replaces the client before native context creation can be reached.
#[cfg(test)]
mod native_reentrancy_tests {
    use super::*;
    use ferroui_base::input::text_input::TextInputMethodClientEvents;
    use ferroui_base::reactive::Disposable;
    use ferroui_base::{Ref, Visual};

    type Callback = RefCell<Option<Box<dyn FnOnce()>>>;

    #[derive(Default)]
    struct ReentrantClient {
        events: TextInputMethodClientEvents,
        connection_callback: Callback,
        text_callback: Callback,
        subscribe_callback: Callback,
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
        fn text_view_visual(&self) -> Option<Ref<Visual>> {
            None
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
}
