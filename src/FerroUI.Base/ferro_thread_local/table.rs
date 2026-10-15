//! The table of the thread-local variables of a thread, and the variables
//! kept in it ([`LocalKey`]): what `ferro_thread_local!` declares on Android
//! (see the parent module).

use std::any::type_name;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::error::Error;
use std::ffi::c_void;
use std::fmt;
use std::mem::needs_drop;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A thread-local variable kept in the table of the thread: what
/// [`ferro_thread_local!`](macro@crate::ferro_thread_local) declares on Android.
/// It has the methods of `std::thread::LocalKey`, with the same contracts.
///
/// As with the standard library, the variable is a constant (so that it can
/// be declared in an `impl` block, and `&NAME` is a `&'static`): it holds a
/// function that returns the static state of the declaration, never a
/// reference to mutable memory.
pub struct LocalKey<T: 'static> {
    state: fn() -> &'static KeyState<T>,
}

/// The static state of a declaration: its index and its initialiser.
#[doc(hidden)]
pub struct KeyState<T: 'static> {
    /// The index in the table plus one; zero until the first use.
    index: AtomicUsize,
    init: fn() -> T,
}

impl<T: 'static> KeyState<T> {
    #[doc(hidden)]
    pub const fn new(init: fn() -> T) -> Self {
        Self { index: AtomicUsize::new(0), init }
    }

    /// The index of the variable in the table of every thread.
    #[inline]
    fn index(&'static self) -> usize {
        let index = self.index.load(Ordering::Relaxed);
        if index != 0 {
            return index - 1;
        }
        self.assign_index()
    }

    #[cold]
    fn assign_index(&'static self) -> usize {
        let candidate = NEXT_INDEX.fetch_add(1, Ordering::Relaxed);
        // Another thread may have assigned one in the meantime; its number
        // wins and the candidate stays unused.
        match self.index.compare_exchange(0, candidate, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => candidate - 1,
            Err(assigned) => assigned - 1,
        }
    }
}

/// The error of [`LocalKey::try_with`]: the variable was destroyed on this
/// thread (the thread is ending). It prints as `std::thread::AccessError`.
#[non_exhaustive]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct AccessError;

impl fmt::Debug for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessError").finish()
    }
}

impl fmt::Display for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt("already destroyed", f)
    }
}

impl Error for AccessError {}

/// The panic of `with` on a destroyed variable: the message of the standard
/// library.
#[inline(never)]
#[cold]
#[track_caller]
fn panic_access_error(err: AccessError) -> ! {
    panic!("cannot access a Thread Local Storage value during or after destruction: {err:?}")
}

/// The source of the indices: zero is "not assigned yet".
static NEXT_INDEX: AtomicUsize = AtomicUsize::new(1);

impl<T: 'static> LocalKey<T> {
    #[doc(hidden)]
    pub const fn new(state: fn() -> &'static KeyState<T>) -> Self {
        Self { state }
    }

    /// The value of this thread, created first if it does not exist yet (from
    /// `provided` when that holds a value, else by the initialiser); null when
    /// the variable was destroyed on this thread.
    ///
    /// The path of a value that exists is short and inlined even without
    /// optimisation (a development build of an application does every property
    /// access through it): the index, the key, the table, the slot.
    #[inline(always)]
    fn get_or_create(&'static self, provided: Option<&mut Option<T>>) -> *const T {
        let state = (self.state)();
        let index = state.index.load(Ordering::Relaxed);
        if index != 0 {
            let value = alive_value(index - 1);
            if !value.is_null() {
                return value.cast_const().cast();
            }
        }
        self.get_or_create_slow(state, provided)
    }

    /// `get_or_create` when the value does not exist yet on this thread, or
    /// was destroyed.
    #[cold]
    #[inline(never)]
    fn get_or_create_slow(&'static self, state: &'static KeyState<T>, provided: Option<&mut Option<T>>) -> *const T {
        let index = state.index();
        // SAFETY: the table belongs to this thread and outlives every call made on it
        // (it is freed by the destructor of the key, after the last value).
        let Some(table) = (unsafe { current_table() }) else {
            return ptr::null();
        };
        match table.lookup(index) {
            Lookup::Alive(value) => return value.cast_const().cast(),
            Lookup::Destroyed => return ptr::null(),
            Lookup::Empty => {}
        }
        // No borrow of the table is held while the initialiser runs: it may use other
        // variables, which may grow the table.
        let value = provided.and_then(Option::take).unwrap_or_else(state.init);
        let value = Box::into_raw(Box::new(value));
        let slot = Slot { value: value.cast(), free: free_value::<T>, needs_drop: needs_drop::<T>() };
        match table.store(index, slot) {
            Stored::New => value,
            Stored::Replaced(previous) => {
                // The initialiser used the variable itself, which created a value; the
                // one returned last wins, as with `thread_local!`. Nobody can hold a
                // reference to the previous one: the `with` that created it has returned.
                // SAFETY: the slot was taken out of the table, so this is its only owner.
                unsafe { (previous.free)(previous.value) };
                value
            }
            Stored::Refused => {
                // SAFETY: the value was not stored.
                unsafe { free_value::<T>(value.cast()) };
                ptr::null()
            }
        }
    }

    /// Calls `f` with a reference to the value of this thread, creating the
    /// value first if this thread has not used the variable yet.
    ///
    /// # Panics
    /// Panics when the variable was destroyed on this thread (the thread is
    /// ending), with the message of `std::thread::LocalKey::with`.
    #[inline(always)]
    pub fn with<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        match self.try_with(f) {
            Ok(result) => result,
            Err(err) => panic_access_error(err),
        }
    }

    /// Calls `f` with a reference to the value of this thread, creating the
    /// value first if this thread has not used the variable yet; an
    /// [`AccessError`] when the variable was destroyed on this thread.
    #[inline(always)]
    pub fn try_with<F, R>(&'static self, f: F) -> Result<R, AccessError>
    where
        F: FnOnce(&T) -> R,
    {
        let value = self.get_or_create(None);
        if value.is_null() {
            return Err(AccessError);
        }
        // SAFETY: the value is boxed and stays where it is until it is destroyed, which
        // happens on this thread after every `with` has returned.
        Ok(f(unsafe { &*value }))
    }

    /// Calls `f` with the value of this thread, created from `init` when the
    /// thread has none yet (the initialiser does not run); `f` receives
    /// `init` back when it was not used.
    fn initialize_with<F, R>(&'static self, init: T, f: F) -> R
    where
        F: FnOnce(Option<T>, &T) -> R,
    {
        let mut init = Some(init);
        let value = self.get_or_create(Some(&mut init));
        if value.is_null() {
            panic_access_error(AccessError);
        }
        // SAFETY: as in `try_with`.
        f(init, unsafe { &*value })
    }
}

impl<T: 'static> fmt::Debug for LocalKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalKey").field("type", &type_name::<T>()).finish_non_exhaustive()
    }
}

impl<T: 'static> LocalKey<Cell<T>> {
    /// Sets or initialises the value; the initialiser does not run.
    pub fn set(&'static self, value: T) {
        self.initialize_with(Cell::new(value), |value, cell| {
            if let Some(value) = value {
                // The cell existed already, so `value` was not used to create it.
                cell.set(value.into_inner());
            }
        });
    }

    /// A copy of the value.
    pub fn get(&'static self) -> T
    where
        T: Copy,
    {
        self.with(Cell::get)
    }

    /// Takes the value, leaving `Default::default()` in its place.
    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        self.with(Cell::take)
    }

    /// Replaces the value, returning the previous one.
    pub fn replace(&'static self, value: T) -> T {
        self.with(|cell| cell.replace(value))
    }
}

impl<T: 'static> LocalKey<RefCell<T>> {
    /// Calls `f` with a shared borrow of the value.
    pub fn with_borrow<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.with(|cell| f(&cell.borrow()))
    }

    /// Calls `f` with a mutable borrow of the value.
    pub fn with_borrow_mut<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        self.with(|cell| f(&mut cell.borrow_mut()))
    }

    /// Sets or initialises the value; the initialiser does not run.
    pub fn set(&'static self, value: T) {
        self.initialize_with(RefCell::new(value), |value, cell| {
            if let Some(value) = value {
                // The cell existed already, so `value` was not used to create it.
                *cell.borrow_mut() = value.into_inner();
            }
        });
    }

    /// Takes the value, leaving `Default::default()` in its place.
    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        self.with(RefCell::take)
    }

    /// Replaces the value, returning the previous one.
    pub fn replace(&'static self, value: T) -> T {
        self.with(|cell| cell.replace(value))
    }
}

// --- The table ---------------------------------------------------------------------------

/// A value in the table: a box whose type the slot no longer knows.
#[derive(Clone, Copy)]
struct Slot {
    value: *mut u8,
    /// Drops the box (`free_value::<T>`).
    free: unsafe fn(*mut u8),
    /// Whether dropping it runs code (`needs_drop::<T>()`): only those are
    /// destroyed in order, and become unusable once destroyed.
    needs_drop: bool,
}

/// Drops a box made by `LocalKey::<T>::get_or_create`.
///
/// # Safety
/// `value` is a `Box<T>` turned into a pointer, owned by the caller.
unsafe fn free_value<T>(value: *mut u8) {
    drop(unsafe { Box::from_raw(value.cast::<T>()) });
}

#[derive(Clone, Copy)]
enum State {
    Empty,
    Alive(Slot),
    Destroyed,
}

enum Lookup {
    Empty,
    Alive(*mut u8),
    Destroyed,
}

enum Stored {
    New,
    Replaced(Slot),
    Refused,
}

/// The table of a thread. Its methods are short and run no code of a value,
/// so the cells are never borrowed while an initialiser or a destructor runs.
struct Table {
    slots: UnsafeCell<Vec<State>>,
    /// The indices of the values that have a destructor, in the order they
    /// were created.
    created: UnsafeCell<Vec<usize>>,
}

impl Table {
    fn new() -> Self {
        Self { slots: UnsafeCell::new(Vec::new()), created: UnsafeCell::new(Vec::new()) }
    }

    fn lookup(&self, index: usize) -> Lookup {
        // SAFETY: the borrow ends in this function, which calls nothing of a value.
        let slots = unsafe { &*self.slots.get() };
        match slots.get(index) {
            None | Some(State::Empty) => Lookup::Empty,
            Some(State::Alive(slot)) => Lookup::Alive(slot.value),
            Some(State::Destroyed) => Lookup::Destroyed,
        }
    }

    fn store(&self, index: usize, slot: Slot) -> Stored {
        // SAFETY: as in `lookup`.
        let slots = unsafe { &mut *self.slots.get() };
        if slots.len() <= index {
            slots.resize(index + 1, State::Empty);
        }
        match slots[index] {
            State::Empty => {
                slots[index] = State::Alive(slot);
                if slot.needs_drop {
                    // SAFETY: as in `lookup`; the other cell.
                    unsafe { &mut *self.created.get() }.push(index);
                }
                Stored::New
            }
            State::Alive(previous) => {
                slots[index] = State::Alive(slot);
                Stored::Replaced(previous)
            }
            State::Destroyed => Stored::Refused,
        }
    }

    /// Takes the value created last of those with a destructor, and marks its
    /// variable destroyed.
    fn take_last_created(&self) -> Option<Slot> {
        // SAFETY: as in `lookup`.
        let created = unsafe { &mut *self.created.get() };
        let slots = unsafe { &mut *self.slots.get() };
        while let Some(index) = created.pop() {
            if let State::Alive(slot) = std::mem::replace(&mut slots[index], State::Destroyed) {
                return Some(slot);
            }
        }
        None
    }
}

impl Drop for Table {
    /// Frees what is left: values without a destructor (and, for a table that
    /// was not destroyed through its key, any other).
    fn drop(&mut self) {
        for state in self.slots.get_mut().drain(..) {
            if let State::Alive(slot) = state {
                // SAFETY: the table owns the value.
                unsafe { (slot.free)(slot.value) };
            }
        }
    }
}

// --- The key of the C library ------------------------------------------------------------

#[cfg(target_os = "android")]
type PthreadKey = std::ffi::c_int;
#[cfg(target_os = "linux")]
type PthreadKey = std::ffi::c_uint;
#[cfg(any(target_os = "macos", target_os = "ios"))]
type PthreadKey = std::ffi::c_ulong;
#[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos", target_os = "ios")))]
compile_error!("the table of thread-local variables is written for Android, Linux and Apple systems");

extern "C" {
    fn pthread_key_create(key: *mut PthreadKey, destructor: Option<unsafe extern "C" fn(*mut c_void)>) -> i32;
    fn pthread_key_delete(key: PthreadKey) -> i32;
    fn pthread_getspecific(key: PthreadKey) -> *mut c_void;
    fn pthread_setspecific(key: PthreadKey, value: *const c_void) -> i32;
}

/// The key, plus one; zero until it is created.
static KEY: AtomicUsize = AtomicUsize::new(0);

/// The value of the key of a thread whose table was destroyed.
const DESTROYED: *mut c_void = 1 as *mut c_void;

/// The value at `index` of the table of this thread, when the thread has a
/// table and the value exists; null otherwise (the slow path decides).
#[inline(always)]
fn alive_value(index: usize) -> *mut u8 {
    let key = KEY.load(Ordering::Acquire);
    if key == 0 {
        return ptr::null_mut();
    }
    // SAFETY: a key of this module.
    let table = unsafe { pthread_getspecific((key - 1) as PthreadKey) };
    if table.is_null() || table == DESTROYED {
        return ptr::null_mut();
    }
    // SAFETY: the key holds a table of this thread until it is destroyed; the borrow ends
    // here and calls nothing of a value.
    let slots = unsafe { &*(*table.cast::<Table>()).slots.get() };
    match slots.get(index) {
        Some(State::Alive(slot)) => slot.value,
        _ => ptr::null_mut(),
    }
}

fn key() -> PthreadKey {
    match KEY.load(Ordering::Acquire) {
        0 => create_key(),
        key => (key - 1) as PthreadKey,
    }
}

#[cold]
fn create_key() -> PthreadKey {
    let mut key: PthreadKey = 0;
    // SAFETY: the call writes one key.
    let status = unsafe { pthread_key_create(&mut key, Some(destroy_thread_table)) };
    if status != 0 {
        // Nothing of a thread-local variable can work without it.
        std::process::abort();
    }
    let stored = key as usize + 1;
    match KEY.compare_exchange(0, stored, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => key,
        Err(existing) => {
            // Another thread created one first.
            // SAFETY: the key is this function's own and was never used.
            unsafe { pthread_key_delete(key) };
            (existing - 1) as PthreadKey
        }
    }
}

/// The table of the calling thread, created if it has none; `None` once it
/// was destroyed (the thread is ending).
///
/// # Safety
/// The reference is used on this thread only and not after the thread ended.
unsafe fn current_table() -> Option<&'static Table> {
    let key = key();
    // SAFETY: a key of this module.
    let table = unsafe { pthread_getspecific(key) };
    if table == DESTROYED {
        return None;
    }
    if !table.is_null() {
        // SAFETY: the key holds a table of this thread until it is destroyed.
        return Some(unsafe { &*table.cast::<Table>() });
    }
    let table = Box::into_raw(Box::new(Table::new()));
    // SAFETY: the key of this module, and a pointer the destructor frees.
    if unsafe { pthread_setspecific(key, table.cast_const().cast()) } != 0 {
        std::process::abort();
    }
    // SAFETY: just created.
    Some(unsafe { &*table })
}

/// The destructor of the key: the thread is ending.
unsafe extern "C" fn destroy_thread_table(table: *mut c_void) {
    let key = key();
    if table == DESTROYED {
        // A later round of the destructors of the C library (it cleared the value
        // before calling): the table stays destroyed for whatever destructor of another
        // key still runs.
        // SAFETY: the key of this module.
        unsafe { pthread_setspecific(key, DESTROYED) };
        return;
    }
    // The C library cleared the value before calling: the table is set again, so that
    // the destructors of the values find it (and the variables that are not destroyed
    // yet).
    // SAFETY: the key of this module, and the table of this thread.
    unsafe {
        pthread_setspecific(key, table);
        destroy_table_of_thread(table.cast());
        pthread_setspecific(key, DESTROYED);
    }
}

/// # Safety
/// `table` is the table of the calling thread, set in the key.
unsafe fn destroy_table_of_thread(table: *mut Table) {
    // SAFETY: the caller's contract. The values are destroyed while the key still holds
    // the table; it is freed (with what has no destructor) at the end.
    unsafe {
        let shared = &*table;
        while let Some(slot) = shared.take_last_created() {
            (slot.free)(slot.value);
        }
        drop(Box::from_raw(table));
    }
}
