//! Tests of the table of thread-local variables that `ferro_thread_local!`
//! uses on Android (`ferro_thread_local/table.rs`). The variables are
//! declared with the table form of the macro directly, so the table is
//! exercised on the host whatever the target.

use crate::ferro_thread_local::table::AccessError;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;

/// The panic message of a closure, as a string.
fn panic_message(f: impl FnOnce()) -> String {
    let payload = catch_unwind(AssertUnwindSafe(f)).expect_err("expected a panic");
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else {
        String::from("<not a string>")
    }
}

#[test]
fn the_initialiser_runs_on_first_use_once_per_thread() {
    static RUNS: AtomicUsize = AtomicUsize::new(0);
    fn counted() -> Cell<u32> {
        RUNS.fetch_add(1, Ordering::SeqCst);
        Cell::new(7)
    }
    crate::__ferro_thread_local_table! {
        static VALUE: Cell<u32> = counted();
    }

    thread::spawn(|| {
        assert_eq!(RUNS.load(Ordering::SeqCst), 0, "declaring the variable runs nothing");
        assert_eq!(VALUE.get(), 7);
        assert_eq!(RUNS.load(Ordering::SeqCst), 1);
        VALUE.set(8);
        assert_eq!(VALUE.get(), 8);
        assert_eq!(RUNS.load(Ordering::SeqCst), 1, "a second use does not initialise again");
    })
    .join()
    .unwrap();

    thread::spawn(|| {
        // `set` before any other use creates the value without the initialiser.
        VALUE.set(3);
        assert_eq!(RUNS.load(Ordering::SeqCst), 1);
        assert_eq!(VALUE.get(), 3);
    })
    .join()
    .unwrap();
}

#[test]
fn every_thread_has_its_own_value() {
    crate::__ferro_thread_local_table! {
        static VALUE: RefCell<Vec<u32>> = RefCell::new(vec![1]);
    }

    VALUE.with_borrow_mut(|values| values.push(2));
    let other = thread::spawn(|| {
        VALUE.with_borrow_mut(|values| values.push(3));
        VALUE.with_borrow(Clone::clone)
    })
    .join()
    .unwrap();
    assert_eq!(other, vec![1, 3]);
    assert_eq!(VALUE.with_borrow(Clone::clone), vec![1, 2]);
}

#[test]
fn the_methods_of_cells_behave_as_those_of_the_standard_library() {
    crate::__ferro_thread_local_table! {
        static NUMBER: Cell<i32> = const { Cell::new(1) };
        #[allow(dead_code)]
        pub(crate) static TEXT: RefCell<String> = RefCell::new(String::from("a"));
        static OPTION: Cell<Option<i32>> = const { Cell::new(Some(5)) }
    }

    assert_eq!(NUMBER.replace(2), 1);
    assert_eq!(NUMBER.take(), 2);
    assert_eq!(NUMBER.get(), 0);
    assert_eq!(OPTION.take(), Some(5));
    assert_eq!(OPTION.get(), None);

    assert_eq!(TEXT.replace(String::from("b")), "a");
    TEXT.set(String::from("c"));
    assert_eq!(TEXT.with_borrow(Clone::clone), "c");
    assert_eq!(TEXT.take(), "c");
    assert_eq!(TEXT.with_borrow(String::len), 0);
    assert_eq!(TEXT.try_with(|text| text.borrow().len()), Ok(0));
}

#[test]
fn an_initialiser_may_use_other_variables() {
    // The variables are first used inside one another's initialiser, so the table grows
    // while an initialiser runs.
    crate::__ferro_thread_local_table! {
        static OUTER: u32 = INNER.with(|inner| inner + 1);
        static INNER: u32 = DEEPEST.with(|deepest| deepest * 10);
        static DEEPEST: u32 = 4;
    }

    thread::spawn(|| {
        assert_eq!(OUTER.with(|outer| *outer), 41);
        assert_eq!(INNER.with(|inner| *inner), 40);
        assert_eq!(DEEPEST.with(|deepest| *deepest), 4);
    })
    .join()
    .unwrap();
}

#[test]
fn an_initialiser_that_uses_its_own_variable_wins() {
    static DROPPED: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    struct Recorded(u32);
    impl Drop for Recorded {
        fn drop(&mut self) {
            DROPPED.lock().unwrap().push(self.0);
        }
    }
    static DEPTH: AtomicUsize = AtomicUsize::new(0);
    fn create() -> Recorded {
        if DEPTH.fetch_add(1, Ordering::SeqCst) == 0 {
            // The inner use creates a value first; the outer one replaces it, as with
            // `thread_local!`.
            assert_eq!(VALUE.with(|value| value.0), 2);
            Recorded(1)
        } else {
            Recorded(2)
        }
    }
    crate::__ferro_thread_local_table! {
        static VALUE: Recorded = create();
    }

    thread::spawn(|| {
        assert_eq!(VALUE.with(|value| value.0), 1);
        assert_eq!(*DROPPED.lock().unwrap(), vec![2], "the replaced value is dropped");
    })
    .join()
    .unwrap();
    assert_eq!(*DROPPED.lock().unwrap(), vec![2, 1]);
}

#[test]
fn values_are_destroyed_in_the_reverse_order_of_their_creation() {
    static LOG: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    struct Logged(&'static str);
    impl Drop for Logged {
        fn drop(&mut self) {
            LOG.lock().unwrap().push(self.0);
        }
    }
    crate::__ferro_thread_local_table! {
        static FIRST: Logged = Logged("first");
        static SECOND: Logged = Logged("second");
        static THIRD: Logged = Logged("third");
    }

    thread::spawn(|| {
        // Created in another order than declared: the order of creation counts.
        SECOND.with(|_| ());
        THIRD.with(|_| ());
        FIRST.with(|_| ());
        assert!(LOG.lock().unwrap().is_empty());
    })
    .join()
    .unwrap();
    assert_eq!(*LOG.lock().unwrap(), vec!["first", "third", "second"]);
}

#[test]
fn a_destructor_may_use_variables_that_are_not_destroyed_yet() {
    static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());
    fn log(entry: String) {
        LOG.lock().unwrap().push(entry);
    }

    struct Early;
    impl Drop for Early {
        fn drop(&mut self) {
            log(String::from("early dropped"));
        }
    }
    struct Late;
    impl Drop for Late {
        fn drop(&mut self) {
            log(String::from("late dropped"));
        }
    }
    struct Created;
    impl Drop for Created {
        fn drop(&mut self) {
            log(String::from("created during destruction dropped"));
        }
    }
    struct Middle;
    impl Drop for Middle {
        fn drop(&mut self) {
            // Created before this one: not destroyed yet.
            log(format!("early {:?}", EARLY.try_with(|_| ())));
            // Created after this one: destroyed already.
            log(format!("late {:?}", LATE.try_with(|_| ())));
            let message = panic_message(|| LATE.with(|_| ()));
            log(format!("late with: {message}"));
            // Without a destructor: usable until the end, even when created after.
            log(format!("plain {:?}", PLAIN.try_with(Cell::get)));
            // Created now: destroyed in its turn.
            log(format!("created {:?}", CREATED.try_with(|_| ())));
        }
    }
    crate::__ferro_thread_local_table! {
        static EARLY: Early = Early;
        static MIDDLE: Middle = Middle;
        static LATE: Late = Late;
        static PLAIN: Cell<u32> = const { Cell::new(9) };
        static CREATED: Created = Created;
    }

    thread::spawn(|| {
        EARLY.with(|_| ());
        MIDDLE.with(|_| ());
        LATE.with(|_| ());
        PLAIN.with(|_| ());
    })
    .join()
    .unwrap();

    let message = "cannot access a Thread Local Storage value during or after destruction: AccessError";
    assert_eq!(
        *LOG.lock().unwrap(),
        vec![
            String::from("late dropped"),
            String::from("early Ok(())"),
            String::from("late Err(AccessError)"),
            format!("late with: {message}"),
            String::from("plain Ok(9)"),
            String::from("created Ok(())"),
            String::from("created during destruction dropped"),
            String::from("early dropped"),
        ]
    );
}

#[test]
fn the_access_error_and_the_panic_are_those_of_the_standard_library() {
    // A variable of the standard library whose destructor uses itself sees the error of
    // the standard library; it is compared with the one of the table.
    static STD: Mutex<Option<(String, String, String)>> = Mutex::new(None);
    struct UsesItself;
    impl Drop for UsesItself {
        fn drop(&mut self) {
            let error = SELF.try_with(|_| ()).expect_err("destroyed");
            let message = panic_message(|| SELF.with(|_| ()));
            *STD.lock().unwrap() = Some((format!("{error:?}"), error.to_string(), message));
        }
    }
    ::std::thread_local! {
        static SELF: UsesItself = const { UsesItself };
    }
    thread::spawn(|| SELF.with(|_| ())).join().unwrap();
    let (debug, display, message) = STD.lock().unwrap().take().expect("the destructor ran");

    static TABLE: Mutex<Option<String>> = Mutex::new(None);
    struct UsesTableItself;
    impl Drop for UsesTableItself {
        fn drop(&mut self) {
            assert_eq!(TABLE_SELF.try_with(|_| ()), Err(AccessError));
            *TABLE.lock().unwrap() = Some(panic_message(|| TABLE_SELF.with(|_| ())));
        }
    }
    crate::__ferro_thread_local_table! {
        static TABLE_SELF: UsesTableItself = const { UsesTableItself };
    }
    thread::spawn(|| TABLE_SELF.with(|_| ())).join().unwrap();

    assert_eq!(format!("{AccessError:?}"), debug);
    assert_eq!(AccessError.to_string(), display);
    assert_eq!(TABLE.lock().unwrap().take().expect("the destructor ran"), message);
}

#[test]
fn many_variables_fit_one_table() {
    // More variables than the C library has keys for a process.
    macro_rules! counters {
        ($keys:ident; $($name:ident)*) => {
            // One declaration per invocation: a block of declarations is expanded
            // recursively, as by `thread_local!`, and 140 would pass the recursion limit.
            $(crate::__ferro_thread_local_table! {
                static $name: Cell<usize> = const { Cell::new(0) };
            })*
            let $keys = [$(&$name),*];
        };
    }
    fn run() {
        counters! {
            keys;
            A0 A1 A2 A3 A4 A5 A6 A7 A8 A9 B0 B1 B2 B3 B4 B5 B6 B7 B8 B9
            C0 C1 C2 C3 C4 C5 C6 C7 C8 C9 D0 D1 D2 D3 D4 D5 D6 D7 D8 D9
            E0 E1 E2 E3 E4 E5 E6 E7 E8 E9 F0 F1 F2 F3 F4 F5 F6 F7 F8 F9
            G0 G1 G2 G3 G4 G5 G6 G7 G8 G9 H0 H1 H2 H3 H4 H5 H6 H7 H8 H9
            I0 I1 I2 I3 I4 I5 I6 I7 I8 I9 J0 J1 J2 J3 J4 J5 J6 J7 J8 J9
            K0 K1 K2 K3 K4 K5 K6 K7 K8 K9 L0 L1 L2 L3 L4 L5 L6 L7 L8 L9
            M0 M1 M2 M3 M4 M5 M6 M7 M8 M9 N0 N1 N2 N3 N4 N5 N6 N7 N8 N9
        }
        for (i, key) in keys.iter().enumerate() {
            key.set(i);
        }
        for (i, key) in keys.iter().enumerate() {
            assert_eq!(key.get(), i);
        }
    }
    thread::spawn(run).join().unwrap();
    thread::spawn(run).join().unwrap();
}

#[test]
fn a_variable_is_a_constant_as_with_the_standard_library() {
    // Declared in an `impl` block, as an associated constant, and passed on as a
    // `&'static LocalKey`.
    struct Owner;
    impl Owner {
        crate::__ferro_thread_local_table! {
            static COUNT: Cell<u32> = const { Cell::new(0) };
        }
    }
    fn add(key: &'static crate::ferro_thread_local::table::LocalKey<Cell<u32>>, amount: u32) {
        key.set(key.get() + amount);
    }

    add(&Owner::COUNT, 2);
    add(&Owner::COUNT, 3);
    assert_eq!(Owner::COUNT.get(), 5);
}
