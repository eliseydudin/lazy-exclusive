#![allow(non_camel_case_types)]

use core::{cell::UnsafeCell, ptr};
#[cfg(target_os = "windows")]
type SRWLOCK = usize;

#[cfg(target_os = "windows")]
windows_link::link!("kernel32.dll" "system" fn InitializeSRWLock(lock: *mut SRWLOCK));
#[cfg(target_os = "windows")]
windows_link::link!("kernel32.dll" "system" fn AcquireSRWLockExclusive(lock: *mut SRWLOCK));
#[cfg(target_os = "windows")]
windows_link::link!("kernel32.dll" "system" fn ReleaseSRWLockExclusive(lock: *mut SRWLOCK));

#[cfg(not(target_os = "windows"))]
use libc::{
    pthread_mutex_destroy, pthread_mutex_init, pthread_mutex_lock, pthread_mutex_t,
    pthread_mutex_unlock,
};

enum LockState {
    Uninitialized,
    #[cfg(not(target_os = "windows"))]
    Initialized(pthread_mutex_t),
    #[cfg(target_os = "windows")]
    Initialized(SRWLOCK),
}

impl LockState {
    #[cfg(not(target_os = "windows"))]
    pub fn unwrap_initialized(&mut self) -> &mut pthread_mutex_t {
        match self {
            Self::Uninitialized => panic!("The lock's state is uninitialized"),
            Self::Initialized(lock) => lock,
        }
    }

    #[cfg(target_os = "windows")]
    pub fn unwrap_initialized(&mut self) -> &mut SRWLOCK {
        match self {
            Self::Uninitialized => panic!("The lock's state is uninitialized"),
            Self::Initialized(lock) => lock,
        }
    }
}

pub struct Lock(UnsafeCell<LockState>);

impl Lock {
    pub const fn new() -> Self {
        /*
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
        */

        Self(UnsafeCell::new(LockState::Uninitialized))
    }

    fn init(&self) {
        #[cfg(not(target_os = "windows"))]
        let data = unsafe {
            let data = UnsafeCell::new(core::mem::zeroed());
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

        let mutref = unsafe { self.0.get().as_mut() }.expect("Should never fail");
        *mutref = LockState::Initialized(data.into_inner());
    }

    pub fn lock(&self) {
        let mutref = unsafe { self.0.get().as_mut() }.expect("Should never fail");
        let lock = match mutref {
            LockState::Uninitialized => {
                self.init();
                mutref.unwrap_initialized()
            }
            LockState::Initialized(lock) => lock,
        };

        #[cfg(not(target_os = "windows"))]
        unsafe {
            pthread_mutex_lock(lock as *mut pthread_mutex_t)
        };
        #[cfg(target_os = "windows")]
        unsafe {
            AcquireSRWLockExclusive(lock as *mut SRWLOCK)
        }
    }

    pub fn unlock(&self) {
        let mutref = unsafe { self.0.get().as_mut() }.expect("Should never fail");
        let lock = match mutref {
            LockState::Uninitialized => {
                self.init();
                mutref.unwrap_initialized()
            }
            LockState::Initialized(lock) => lock,
        };

        #[cfg(not(target_os = "windows"))]
        unsafe {
            pthread_mutex_unlock(lock as *mut pthread_mutex_t)
        };
        #[cfg(target_os = "windows")]
        unsafe {
            ReleaseSRWLockExclusive(lock as *mut SRWLOCK)
        }
    }

    pub fn reset(&self) {
        let mutptr = unsafe { self.0.get().as_mut().expect("Should never fail") };
        match mutptr {
            LockState::Uninitialized => (),
            #[cfg(not(target_os = "windows"))]
            LockState::Initialized(lock) => unsafe {
                core::ptr::drop_in_place(lock as *mut pthread_mutex_t);
                *mutptr = LockState::Uninitialized;
            },
            #[cfg(target_os = "windows")]
            _ => (),
        }
    }
}

#[cfg(not(target_os = "windows"))]
impl Drop for Lock {
    fn drop(&mut self) {
        let mutref = unsafe { self.0.get().as_mut() }.expect("Should never fail");
        match mutref {
            LockState::Initialized(lock) => unsafe {
                pthread_mutex_destroy(lock as *mut pthread_mutex_t);
            },
            _ => (),
        }
    }
}

impl Default for Lock {
    fn default() -> Self {
        Self::new()
    }
}
