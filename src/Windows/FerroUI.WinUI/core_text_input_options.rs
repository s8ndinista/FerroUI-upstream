//! Options can arrive before the lazy CoreText context exists. Keep their
//! latest value independently of that context and release all borrows before
//! invoking native setters, which can synchronously replace the input client.
use crate::interop::CoreTextInputScope;
use ferroui_base::input::text_input::{TextInputContentType, TextInputOptions};
use std::cell::{Cell, RefCell};

#[derive(Default)]
pub(crate) struct CoreTextInputOptions {
    options: RefCell<TextInputOptions>,
    revision: Cell<u64>,
    applying: Cell<bool>,
}

struct ApplyingGuard<'a>(&'a Cell<bool>);

impl Drop for ApplyingGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl CoreTextInputOptions {
    pub(crate) fn update(&self, options: &TextInputOptions) {
        if *self.options.borrow() != *options {
            self.options.replace(options.clone());
            self.revision.set(self.revision.get().wrapping_add(1));
        }
    }

    pub(crate) fn apply<E>(
        &self,
        mut set_scope: impl FnMut(CoreTextInputScope) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.applying.replace(true) {
            return Ok(());
        }
        let _guard = ApplyingGuard(&self.applying);
        loop {
            let revision = self.revision.get();
            let scope = match self.options.borrow().content_type {
                TextInputContentType::Email => CoreTextInputScope::EmailAddress,
                TextInputContentType::Number => CoreTextInputScope::Number,
                TextInputContentType::Password => CoreTextInputScope::Password,
                TextInputContentType::Digits => CoreTextInputScope::Digits,
                TextInputContentType::Pin => CoreTextInputScope::PinNumeric,
                TextInputContentType::Name => CoreTextInputScope::PersonalFullName,
                TextInputContentType::Url => CoreTextInputScope::Url,
                TextInputContentType::Search => CoreTextInputScope::Search,
                _ => CoreTextInputScope::Text,
            };
            set_scope(scope)?;
            if revision == self.revision.get() {
                return Ok(());
            }
            // A native callback changed the options while the previous setter
            // was in flight. Finish that call before applying the latest scope.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(content_type: TextInputContentType) -> TextInputOptions {
        TextInputOptions {
            content_type,
            ..TextInputOptions::default()
        }
    }

    #[test]
    fn core_text_first_context_uses_the_last_options_received_before_creation() {
        let pending = CoreTextInputOptions::default();
        pending.update(&options(TextInputContentType::Url));
        pending.update(&options(TextInputContentType::Email));
        let native_calls = RefCell::new(Vec::new());
        // There is no native context when the manager first calls set_options.
        // Its first client later creates one and applies the retained options.
        pending
            .apply(|scope| {
                native_calls.borrow_mut().push(scope);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(*native_calls.borrow(), [CoreTextInputScope::EmailAddress]);
    }

    #[test]
    fn core_text_options_reentrant_replacement_wins_after_the_old_native_setter() {
        let pending = CoreTextInputOptions::default();
        pending.update(&options(TextInputContentType::Email));
        let calls = RefCell::new(Vec::new());
        let native_scope = Cell::new(CoreTextInputScope::Text);
        pending
            .apply(|scope| {
                calls.borrow_mut().push(scope);
                if scope == CoreTextInputScope::EmailAddress {
                    pending.update(&options(TextInputContentType::Search));
                    pending
                        .apply(|_| -> Result<(), ()> {
                            panic!("a nested setter must wait for the old native call")
                        })
                        .unwrap();
                }
                // The old native setter completes after its reentrant callback.
                native_scope.set(scope);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(
            *calls.borrow(),
            [CoreTextInputScope::EmailAddress, CoreTextInputScope::Search]
        );
        assert_eq!(native_scope.get(), CoreTextInputScope::Search);
    }

    #[test]
    fn core_text_failed_option_application_preserves_the_latest_value_for_retry() {
        let pending = CoreTextInputOptions::default();
        pending.update(&options(TextInputContentType::Digits));
        assert_eq!(
            pending.apply(|scope| {
                assert_eq!(scope, CoreTextInputScope::Digits);
                pending.update(&options(TextInputContentType::Number));
                Err::<(), _>("native failure")
            }),
            Err("native failure")
        );
        let native_scope = Cell::new(CoreTextInputScope::Text);
        pending
            .apply(|scope| {
                native_scope.set(scope);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(native_scope.get(), CoreTextInputScope::Number);
    }

    #[test]
    fn core_text_content_types_preserve_native_name_and_numeric_pin_semantics() {
        // These are keyboard/input-processor hints; the adapter does not add
        // text validation or impose the native scope's typical PIN length.
        let cases = [
            (TextInputContentType::Normal, CoreTextInputScope::Text),
            (TextInputContentType::Alpha, CoreTextInputScope::Text),
            (TextInputContentType::Digits, CoreTextInputScope::Digits),
            (TextInputContentType::Pin, CoreTextInputScope::PinNumeric),
            (TextInputContentType::Number, CoreTextInputScope::Number),
            (
                TextInputContentType::Email,
                CoreTextInputScope::EmailAddress,
            ),
            (TextInputContentType::Url, CoreTextInputScope::Url),
            (
                TextInputContentType::Name,
                CoreTextInputScope::PersonalFullName,
            ),
            (TextInputContentType::Password, CoreTextInputScope::Password),
            (TextInputContentType::Social, CoreTextInputScope::Text),
            (TextInputContentType::Search, CoreTextInputScope::Search),
        ];
        for (content_type, expected) in cases {
            let pending = CoreTextInputOptions::default();
            pending.update(&options(content_type));
            let received = Cell::new(None);
            pending
                .apply(|scope| {
                    received.set(Some(scope));
                    Ok::<_, ()>(())
                })
                .unwrap();
            assert_eq!(received.get(), Some(expected), "{content_type:?}");
        }
    }
}
