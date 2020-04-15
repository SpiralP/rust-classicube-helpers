use std::{
    cell::RefCell,
    sync::{Mutex, RwLock},
    thread::LocalKey,
};

pub trait WithInner<O> {
    fn with_inner<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&O) -> T;

    fn with_inner_mut<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&mut O) -> T;
}

impl<O> WithInner<O> for LocalKey<RefCell<Option<O>>> {
    fn with_inner<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&O) -> T,
    {
        self.with(|cell| {
            if let Some(inner) = &*cell.borrow() {
                Some(f(inner))
            } else {
                None
            }
        })
    }

    fn with_inner_mut<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&mut O) -> T,
    {
        self.with(|cell| {
            if let Some(inner) = &mut *cell.borrow_mut() {
                Some(f(inner))
            } else {
                None
            }
        })
    }
}

impl<O> WithInner<O> for Mutex<Option<O>> {
    fn with_inner<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&O) -> T,
    {
        if let Some(inner) = &*self.lock().unwrap() {
            Some(f(inner))
        } else {
            None
        }
    }

    fn with_inner_mut<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&mut O) -> T,
    {
        if let Some(inner) = &mut *self.lock().unwrap() {
            Some(f(inner))
        } else {
            None
        }
    }
}

impl<O> WithInner<O> for RwLock<Option<O>> {
    fn with_inner<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&O) -> T,
    {
        if let Some(inner) = &*self.read().unwrap() {
            Some(f(inner))
        } else {
            None
        }
    }

    fn with_inner_mut<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&mut O) -> T,
    {
        if let Some(inner) = &mut *self.write().unwrap() {
            Some(f(inner))
        } else {
            None
        }
    }
}
