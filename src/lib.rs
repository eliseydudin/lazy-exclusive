use std::{
    cell::{Cell, UnsafeCell},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Copy)]
enum State {
    Unlocked,
    Locked,
}

pub struct LazyExclusive<T> {
    state: Cell<State>,
    data: UnsafeCell<T>,
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

        Self { state, data }
    }

    pub fn get(&self) -> Option<Mut<'_, T>> {
        match self.state.get() {
            State::Unlocked => {
                self.state.set(State::Locked);
                Some(Mut { source: self })
            }
            State::Locked => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::LazyExclusive;

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
}
