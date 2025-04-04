#![allow(non_camel_case_types)]

use std::{cell::UnsafeCell, ptr};
#[cfg(target_os = "windows")]
type SRWLOCK = usize;

#[cfg(target_os = "windows")]
windows_link::link!("kernel32.dll" "system" fn InitializeSRWLock(lock: *mut SRWLOCK));
#[cfg(target_os = "windows")]
windows_link::link!("kernel32.dll" "system" fn AcquireSRWLockExclusive(lock: *mut SRWLOCK));
#[cfg(target_os = "windows")]
windows_link::link!("kernel32.dll" "system" fn ReleaseSRWLockExclusive(lock: *mut SRWLOCK));

#[cfg(all(target_pointer_width = "64", not(target_os = "windows")))]
type PTHREAD_MUTEX_T = [u8; 40];
#[cfg(all(target_pointer_width = "64", not(target_os = "windows")))]
const LEN: usize = 40;

#[cfg(all(target_pointer_width = "32", not(target_os = "windows")))]
type PTHREAD_MUTEX_T = [u8; 24];
#[cfg(all(target_pointer_width = "32", not(target_os = "windows")))]
const LEN: usize = 24;

#[cfg(not(target_os = "windows"))]
#[link(name = "pthread")]
unsafe extern "C" {
    fn pthread_mutex_init(lock: *mut PTHREAD_MUTEX_T, attr: *const u8) -> i32;
    fn pthread_mutex_lock(lock: *mut PTHREAD_MUTEX_T) -> i32;
    fn pthread_mutex_unlock(lock: *mut PTHREAD_MUTEX_T) -> i32;
    fn pthread_mutex_destroy(lock: *mut PTHREAD_MUTEX_T) -> i32;
}

pub struct Lock {
    #[cfg(not(target_os = "windows"))]
    data: UnsafeCell<PTHREAD_MUTEX_T>,
    #[cfg(target_os = "windows")]
    data: UnsafeCell<SRWLOCK>,
}

impl Lock {
    pub fn new() -> Self {
        #[cfg(not(target_os = "windows"))]
        let data = unsafe {
            let data = UnsafeCell::new([0_u8; LEN]);
            let result = pthread_mutex_init(data.get(), ptr::null());
            assert_eq!(
                result, 0,
                "Cannot initialize the mutex: `pthread_mutex_init` returned a non-zero value"
            );
            data
        };
        #[cfg(target_os = "windows")]
        let data = unsafe {
            let cell = UnsafeCell::new(0 as SRWLOCK);
            InitializeSRWLock(cell.get());
            cell
        };

        Self { data }
    }

    pub fn lock(&self) {
        #[cfg(not(target_os = "windows"))]
        unsafe {
            pthread_mutex_lock(self.data.get())
        };
        #[cfg(target_os = "windows")]
        unsafe {
            AcquireSRWLockExclusive(self.data.get())
        }
    }

    pub fn unlock(&self) {
        #[cfg(not(target_os = "windows"))]
        unsafe {
            pthread_mutex_unlock(self.data.get())
        };
        #[cfg(target_os = "windows")]
        unsafe {
            ReleaseSRWLockExclusive(self.data.get())
        }
    }
}

#[cfg(not(target_os = "windows"))]
impl Drop for Lock {
    fn drop(&mut self) {
        unsafe { pthread_mutex_destroy(self.data.get()) };
    }
}
