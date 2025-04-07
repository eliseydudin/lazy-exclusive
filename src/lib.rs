#![no_std]

use core::{
    cell::UnsafeCell,
    fmt::{self, Debug},
    ops::{Deref, DerefMut},
};
#[cfg(feature = "use-locks")]
use lock::Lock;

#[cfg(feature = "use-locks")]
mod lock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum State {
    Unlocked,
    Locked,
    Poisoned,
}

/// A Cell for [`State`]. Used for const access to its data
pub struct StateCell {
    inner: UnsafeCell<State>,
}

impl StateCell {
    pub const fn new(data: State) -> Self {
        Self {
            inner: UnsafeCell::new(data),
        }
    }

    pub const fn get(&self) -> State {
        // SAFETY: self.inner.get() is never an invalid pointer
        unsafe { *self.inner.get() }
    }

    pub fn set(&self, data: State) {
        let _ = unsafe { core::mem::replace(&mut *self.inner.get(), data) };
    }
}

/// A container type like [`LazyLock`].
/// Allows mutable access, but only one reference at a time.
/// ```rust
/// use lazy_exclusive::LazyExclusive;
///
/// static LAZY: LazyExclusive<i32> = LazyExclusive::new(123);
/// let lock = LAZY.get().unwrap();
/// assert_eq!(*lock, 123);
/// assert!(LAZY.is_locked());
/// ```
///
/// [`LazyLock`]: std::sync::LazyLock
pub struct LazyExclusive<T> {
    state: StateCell,
    data: UnsafeCell<T>,
    #[cfg(feature = "use-locks")]
    lock: Lock,
}

unsafe impl<T> Send for LazyExclusive<T> {}
unsafe impl<T> Sync for LazyExclusive<T> {}

pub struct Mut<'a, T> {
    source: &'a LazyExclusive<T>,
}

impl<T> Mut<'_, T> {
    const fn inner(&mut self) -> &mut T {
        unsafe {
            self.source
                .data
                .get()
                .as_mut()
                .expect("source.data is never a null pointer")
        }
    }
}

impl<T> Drop for Mut<'_, T> {
    fn drop(&mut self) {
        self.source.state.set(State::Unlocked);
        #[cfg(feature = "use-locks")]
        {
            self.source.lock.unlock();

            #[cfg(feature = "std")]
            {
                extern crate std;
                if std::thread::panicking() {
                    self.source.state.set(State::Poisoned);
                }
            }
        }
    }
}

impl<T> Deref for Mut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            self.source
                .data
                .get()
                .as_ref()
                .expect("source.data is never a null pointer")
        }
    }
}

impl<T> AsRef<T> for Mut<'_, T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T> AsMut<T> for Mut<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

impl<T> DerefMut for Mut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner()
    }
}

impl<T> LazyExclusive<T> {
    pub const fn new(data: T) -> Self {
        let data = UnsafeCell::new(data);
        let state = StateCell::new(State::Unlocked);

        #[cfg(not(feature = "use-locks"))]
        return Self { state, data };
        #[cfg(feature = "use-locks")]
        Self {
            state,
            data,
            lock: Lock::new(),
        }
    }

    /// Get a handle to the inner data. Returns [`None`] if a handle already exists
    pub fn get(&self) -> Option<Mut<'_, T>> {
        match self.state.get() {
            State::Unlocked => {
                self.state.set(State::Locked);
                #[cfg(feature = "use-locks")]
                self.lock.lock();
                Some(Mut { source: self })
            }
            _ => None,
        }
    }

    /// Set the inner value to [`new_value`]. Panics if the data is already locked
    pub fn swap(&self, new_value: T) {
        assert_eq!(self.state.get(), State::Unlocked);
        unsafe {
            let t = self.data.get().as_mut().unwrap();
            *t = new_value;
            self.state.set(State::Unlocked);

            #[cfg(feature = "use-locks")]
            self.lock.reset();
        }
    }

    pub const fn get_state(&self) -> State {
        self.state.get()
    }

    /// Wait for the data to unlock and return a new handle
    #[cfg(feature = "use-locks")]
    pub fn wait(&self) -> Mut<'_, T> {
        self.lock.lock();
        assert_eq!(self.state.get(), State::Unlocked, "The data was poisoned");
        self.state.set(State::Locked);
        Mut { source: self }
    }

    pub fn into_inner(self) -> T {
        match self.state.get() {
            State::Unlocked => self.data.into_inner(),
            State::Locked => panic!("locked"),
            State::Poisoned => panic!("poisoned"),
        }
    }

    pub const fn is_unlocked(&self) -> bool {
        matches!(self.state.get(), State::Unlocked)
    }

    pub const fn is_locked(&self) -> bool {
        matches!(self.state.get(), State::Locked)
    }

    pub const fn is_poisoned(&self) -> bool {
        matches!(self.state.get(), State::Poisoned)
    }
}

impl<T: Debug> Debug for LazyExclusive<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data: &dyn Debug = match self.state.get() {
            State::Unlocked => unsafe { self.data.get().as_mut().expect("Should never fail") },
            State::Locked => &"<locked>",
            State::Poisoned => &"<poisoned>",
        };

        f.debug_struct("LazyExclusive")
            .field("state", &self.state.get())
            .field("data", data)
            .finish()
    }
}

impl<T> From<T> for LazyExclusive<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Clone> Clone for LazyExclusive<T> {
    fn clone(&self) -> Self {
        let data = match self.state.get() {
            State::Unlocked => unsafe { self.data.get().as_ref().expect("Should never fail") },
            State::Locked => panic!("locked"),
            State::Poisoned => panic!("poisoned"),
        };

        Self::new(data.clone())
    }
}

impl<T: Default> Default for LazyExclusive<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[cfg(test)]
mod tests {
    use crate::{LazyExclusive, State};

    #[test]
    fn basic() {
        let shared = LazyExclusive::new(230);
        let mut1 = shared.get();
        assert!(mut1.is_some());
        assert!(shared.get().is_none());

        let mut1 = mut1.unwrap();
        let inner = *mut1;
        assert_eq!(inner, 230);
    }

    #[test]
    fn static_test() {
        static SHARED: LazyExclusive<i32> = LazyExclusive::new(1231);
        let pointer = SHARED.get().unwrap();
        assert_eq!(*pointer, 1231);
    }

    #[cfg(all(feature = "use-locks", feature = "std"))]
    #[test]
    fn lock_test() {
        extern crate std;
        use crate::State;
        use std::time::{Duration, Instant};

        let start = Instant::now();
        let five_seconds = Duration::from_secs(5);
        static SHARED: LazyExclusive<i32> = LazyExclusive::new(120);
        let mut lock = SHARED.get().unwrap();

        std::thread::spawn(move || {
            *lock *= 2;
            std::thread::sleep(Duration::new(5, 0));
        });

        assert_eq!(SHARED.get_state(), State::Locked);
        let new_lock = SHARED.wait();
        assert_eq!(*new_lock, 120 * 2);
        assert!(start.elapsed() >= five_seconds);
    }

    #[test]
    fn reset() {
        let lazy = LazyExclusive::new(120);
        lazy.swap(10);
        assert_eq!(*lazy.get().unwrap(), 10);
        assert_eq!(lazy.get_state(), State::Unlocked);
    }

    #[test]
    fn clone() {
        let lazy = LazyExclusive::new(120);
        let clone = lazy.clone();

        assert_eq!(lazy.into_inner(), clone.into_inner());
    }
}
