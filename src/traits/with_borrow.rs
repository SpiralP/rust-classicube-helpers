use std::{
    cell::RefCell,
    sync::{Mutex, RwLock},
    thread::LocalKey,
};

pub trait WithBorrow<O> {
    fn with_borrow<F, T>(&'static self, f: F) -> T
    where
        F: FnOnce(&O) -> T;

    fn with_borrow_mut<F, T>(&'static self, f: F) -> T
    where
        F: FnOnce(&mut O) -> T;
}

impl<O> WithBorrow<O> for LocalKey<RefCell<O>> {
    fn with_borrow<F, T>(&'static self, f: F) -> T
    where
        F: FnOnce(&O) -> T,
    {
        self.with(|cell| f(&cell.borrow()))
    }

    fn with_borrow_mut<F, T>(&'static self, f: F) -> T
    where
        F: FnOnce(&mut O) -> T,
    {
        self.with(|cell| f(&mut cell.borrow_mut()))
    }
}

impl<O> WithBorrow<O> for Mutex<O> {
    fn with_borrow<F, T>(&'static self, f: F) -> T
    where
        F: FnOnce(&O) -> T,
    {
        let guard = self.lock().unwrap();
        f(&*guard)
    }

    fn with_borrow_mut<F, T>(&'static self, f: F) -> T
    where
        F: FnOnce(&mut O) -> T,
    {
        let mut guard = self.lock().unwrap();
        f(&mut *guard)
    }
}

impl<O> WithBorrow<O> for RwLock<O> {
    fn with_borrow<F, T>(&'static self, f: F) -> T
    where
        F: FnOnce(&O) -> T,
    {
        let guard = self.read().unwrap();
        f(&*guard)
    }

    fn with_borrow_mut<F, T>(&'static self, f: F) -> T
    where
        F: FnOnce(&mut O) -> T,
    {
        let mut guard = self.write().unwrap();
        f(&mut *guard)
    }
}

#[test]
fn test_with_borrow_thread_local() {
    thread_local!(
        static THREAD_LOCAL: RefCell<u8> = Default::default();
    );
    assert_eq!(
        THREAD_LOCAL.with_borrow_mut(|o| {
            *o += 2;
            *o
        }),
        2
    );
    assert_eq!(THREAD_LOCAL.with_borrow(|o| o + 2), 4);
}

#[test]
fn test_with_borrow_static_mutex() {
    lazy_static::lazy_static! {
        static ref STATIC_MUTEX: Mutex<u8> = Default::default();
    };
    assert_eq!(
        STATIC_MUTEX.with_borrow_mut(|o| {
            *o += 2;
            *o
        }),
        2
    );
    assert_eq!(STATIC_MUTEX.with_borrow(|o| o + 2), 4);
}

#[test]
fn test_with_borrow_static_rwlock() {
    lazy_static::lazy_static! {
        static ref STATIC_RWLOCK: RwLock<u8> = Default::default();
    };
    assert_eq!(
        STATIC_RWLOCK.with_borrow_mut(|o| {
            *o += 2;
            *o
        }),
        2
    );
    assert_eq!(STATIC_RWLOCK.with_borrow(|o| o + 2), 4);
}
