// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::item_collection_transition::{
    ItemCollectionTransition, ItemCollectionTransitionOperation as Operation,
    ItemCollectionTransitionTriggers as Triggers,
};
use super::item_collection_transition_provider::ItemCollectionTransitionProvider;
use crate::Control;
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Rect, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Derives animation triggers from collection and layout changes during the
/// current layout cycle and returns removed elements when their transition ends.
pub struct TransitionManager {
    provider: RefCell<Option<Ref<ItemCollectionTransitionProvider>>>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    recycle: Rc<dyn Fn(Ref<Control>)>,
    triggers: Cell<Triggers>,
}
impl TransitionManager {
    pub fn new(recycle: Rc<dyn Fn(Ref<Control>)>) -> Rc<Self> {
        Rc::new(Self {
            provider: RefCell::new(None),
            subscription: RefCell::new(None),
            recycle,
            triggers: Cell::new(Triggers::empty()),
        })
    }
    pub fn on_transition_provider_changed(
        self: &Rc<Self>,
        provider: Option<Ref<ItemCollectionTransitionProvider>>,
    ) {
        let old = self.subscription.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        *self.provider.borrow_mut() = provider.clone();
        if let Some(provider) = provider {
            let weak = Rc::downgrade(self);
            let subscription = provider.transition_completed(move |args| {
                if args.transition().operation() == Operation::Remove {
                    if let Some(this) = weak.upgrade() {
                        (this.recycle)(args.element());
                    }
                }
            });
            *self.subscription.borrow_mut() = Some(subscription);
        }
    }
    pub fn on_layout_changing(&self) {
        self.triggers
            .set(self.triggers.get() | Triggers::LAYOUT_TRANSITION);
    }
    pub fn on_items_source_changed(&self, action: NotifyCollectionChangedAction) {
        let added = match action {
            NotifyCollectionChangedAction::Add => Triggers::COLLECTION_CHANGE_ADD,
            NotifyCollectionChangedAction::Remove => Triggers::COLLECTION_CHANGE_REMOVE,
            NotifyCollectionChangedAction::Replace => {
                Triggers::COLLECTION_CHANGE_ADD | Triggers::COLLECTION_CHANGE_REMOVE
            }
            NotifyCollectionChangedAction::Reset => Triggers::COLLECTION_CHANGE_RESET,
            NotifyCollectionChangedAction::Move => Triggers::empty(),
        };
        self.triggers.set(self.triggers.get() | added);
    }
    pub fn on_element_prepared(&self, element: &Ref<Control>) {
        let provider = self.provider.borrow().clone();
        let triggers = self.triggers.get()
            & (Triggers::COLLECTION_CHANGE_ADD
                | Triggers::COLLECTION_CHANGE_RESET
                | Triggers::LAYOUT_TRANSITION);
        if let Some(provider) = provider.filter(|_| !triggers.is_empty()) {
            provider.queue_transition(ItemCollectionTransition::new(
                provider.clone(),
                element.clone(),
                Operation::Add,
                triggers,
            ));
        }
    }
    pub fn clear_element(&self, element: &Ref<Control>) -> bool {
        let provider = self.provider.borrow().clone();
        let triggers = self.triggers.get()
            & (Triggers::COLLECTION_CHANGE_REMOVE | Triggers::COLLECTION_CHANGE_RESET);
        let Some(provider) = provider.filter(|_| !triggers.is_empty()) else {
            return false;
        };
        let transition = ItemCollectionTransition::new(
            provider.clone(),
            element.clone(),
            Operation::Remove,
            triggers,
        );
        if !provider.should_animate(transition.clone()) {
            return false;
        }
        provider.queue_transition(transition);
        true
    }
    pub fn on_element_bounds_changed(
        &self,
        element: &Ref<Control>,
        old_bounds: Rect,
        new_bounds: Rect,
    ) {
        let provider = self.provider.borrow().clone();
        if let Some(provider) = provider {
            let recorded = self.triggers.get();
            let triggers = if recorded.is_empty() {
                Triggers::LAYOUT_TRANSITION
            } else {
                recorded
            };
            provider.queue_transition(ItemCollectionTransition::with_bounds(
                provider.clone(),
                element.clone(),
                triggers,
                old_bounds,
                new_bounds,
            ));
        }
    }
    pub fn on_owner_arranged(&self) {
        self.triggers.set(Triggers::empty());
    }
}
impl Drop for TransitionManager {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}
