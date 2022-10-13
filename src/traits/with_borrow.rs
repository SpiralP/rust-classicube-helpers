use std::{
    cell::RefCell,
    sync::{Mutex, RwLock},
    thread::LocalKey,
};

pub trait WithBorrow<'a, O> {
    fn with_borrow<F, R>(&'a self, f: F) -> R
    where
        F: FnOnce(&O) -> R;

    fn with_borrow_mut<F, R>(&'a self, f: F) -> R
    where
        F: FnOnce(&mut O) -> R;
}

impl<O> WithBorrow<'static, O> for LocalKey<RefCell<O>> {
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

impl<'a, O> WithBorrow<'a, O> for Mutex<O> {
    fn with_borrow<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&O) -> R,
    {
        let guard = self.lock().unwrap();
        f(&*guard)
    }

    fn with_borrow_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut O) -> R,
    {
        let mut guard = self.lock().unwrap();
        f(&mut *guard)
    }
}

impl<'a, O> WithBorrow<'a, O> for RwLock<O> {
    fn with_borrow<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&O) -> R,
    {
        let guard = self.read().unwrap();
        f(&*guard)
    }

    fn with_borrow_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut O) -> R,
    {
        let mut guard = self.write().unwrap();
        f(&mut *guard)
    }
}

#[test]
fn test_with_borrow_thread_local() {
    thread_local!(
        static THREAD_LOCAL: std::cell::RefCell<u8> = Default::default();
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

#[test]
fn test_with_borrow_non_static_mutex() {
    let mutex: Mutex<u8> = Default::default();
    assert_eq!(
        mutex.with_borrow_mut(|o| {
            *o += 2;
            *o
        }),
        2
    );
    assert_eq!(mutex.with_borrow(|o| o + 2), 4);
}
