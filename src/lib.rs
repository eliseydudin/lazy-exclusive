#[cfg(feature = "use-locks")]
use lock::Lock;
use std::{
    cell::{Cell, UnsafeCell},
    ops::{Deref, DerefMut},
};

#[cfg(feature = "use-locks")]
mod lock;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Unlocked,
    Locked,
    Poisoned,
}

pub struct LazyExclusive<T> {
    state: Cell<State>,
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
    pub fn inner(&self) -> &mut T {
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

            if std::thread::panicking() {
                self.source.state.set(State::Poisoned)
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

impl<T> DerefMut for Mut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner()
    }
}

impl<T> LazyExclusive<T> {
    pub const fn new(data: T) -> Self {
        let data = UnsafeCell::new(data);
        let state = Cell::new(State::Unlocked);

        #[cfg(not(feature = "use-locks"))]
        return Self { state, data };
        #[cfg(feature = "use-locks")]
        Self {
            state,
            data,
            lock: Lock::new(),
        }
    }

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

    pub fn swap(&self, new_value: T) {
        unsafe {
            let t = self.data.get().as_mut().unwrap();
            *t = new_value;
            self.state.set(State::Unlocked);

            #[cfg(feature = "use-locks")]
            self.lock.reset();
        }
    }

    pub fn get_state(&self) -> State {
        self.state.get()
    }

    #[cfg(feature = "use-locks")]
    pub fn wait(&self) -> Mut<'_, T> {
        self.lock.lock();
        assert_eq!(self.state.get(), State::Unlocked, "The data was poisoned");
        self.state.set(State::Locked);
        Mut { source: self }
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

    #[cfg(feature = "use-locks")]
    #[test]
    fn lock_test() {
        use crate::State;
        use std::time::Duration;

        static SHARED: LazyExclusive<i32> = LazyExclusive::new(120);
        let mut lock = SHARED.get().unwrap();

        std::thread::spawn(move || {
            *lock *= 2;
            std::thread::sleep(Duration::new(5, 0));
        });

        assert_eq!(SHARED.get_state(), State::Locked);
        let new_lock = SHARED.wait();
        assert_eq!(*new_lock, 120 * 2);
    }

    #[test]
    fn reset() {
        let lazy = LazyExclusive::new(120);
        lazy.swap(10);
        assert_eq!(*lazy.get().unwrap(), 10);
        assert_eq!(lazy.get_state(), State::Unlocked);
    }
}
