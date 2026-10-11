//! Groups raw events that arrive faster than the UI thread handles them
//! (the port of `RawEventGroupingThreadingHelper.cs`).
//!
//! The reference queues from the thread of a backend. Raw events are
//! objects of the UI thread here, so the queue is one of that thread; the
//! order of events and the single post per burst are the reference's.

use crate::raw_event_grouping::{RawEvent, RawEventGrouper};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

struct State {
    grouper: RawEventGrouper,
    raw_queue: RefCell<VecDeque<RawEvent>>,
}

pub struct RawEventGroupingThreadingHelper {
    state: Rc<State>,
}

impl RawEventGroupingThreadingHelper {
    pub fn new(event_callback: impl Fn(RawEvent) + 'static) -> Self {
        Self {
            state: Rc::new(State {
                grouper: RawEventGrouper::new(event_callback, None),
                raw_queue: RefCell::default(),
            }),
        }
    }

    fn queue_handler(state: &State) {
        loop {
            let next = state.raw_queue.borrow_mut().pop_front();
            let Some(next) = next else {
                break;
            };
            state.grouper.handle_event(next);
        }
    }

    pub fn on_event(&self, args: RawEvent) {
        let mut raw_queue = self.state.raw_queue.borrow_mut();
        raw_queue.push_back(args);
        if raw_queue.len() == 1 {
            let state = self.state.clone();
            Dispatcher::ui_thread().post_local(move || Self::queue_handler(&state), DispatcherPriority::INPUT);
        }
    }

    pub fn dispose(&self) {
        let state = self.state.clone();
        Dispatcher::ui_thread().post_local(
            move || state.grouper.dispose(),
            DispatcherPriority::from_value(DispatcherPriority::INPUT.value() + 1),
        );
    }
}
