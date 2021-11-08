use std::{
    cell::{Cell, RefCell},
    sync::{Mutex, RwLock},
    thread::LocalKey,
};

pub trait CellGetSet<T> {
    fn get(&'static self) -> T;
    fn set(&'static self, value: T);
}

impl<T> CellGetSet<T> for LocalKey<Cell<T>>
where
    T: Copy,
{
    fn get(&'static self) -> T {
        self.with(|cell| cell.get())
    }

    fn set(&'static self, value: T) {
        self.with(|cell| cell.set(value))
    }
}

pub trait OptionWithInner<O> {
    #[must_use]
    fn with_inner<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&O) -> T;

    #[must_use]
    fn with_inner_mut<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&mut O) -> T;
}

impl<O> OptionWithInner<O> for LocalKey<RefCell<Option<O>>> {
    #[must_use]
    fn with_inner<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&O) -> T,
    {
        self.with(|cell| cell.borrow().as_ref().map(f))
    }

    #[must_use]
    fn with_inner_mut<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&mut O) -> T,
    {
        self.with(|cell| cell.borrow_mut().as_mut().map(f))
    }
}

impl<O> OptionWithInner<O> for Mutex<Option<O>> {
    #[must_use]
    fn with_inner<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&O) -> T,
    {
        self.lock().unwrap().as_ref().map(f)
    }

    #[must_use]
    fn with_inner_mut<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&mut O) -> T,
    {
        self.lock().unwrap().as_mut().map(f)
    }
}

impl<O> OptionWithInner<O> for RwLock<Option<O>> {
    #[must_use]
    fn with_inner<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&O) -> T,
    {
        self.read().unwrap().as_ref().map(f)
    }

    #[must_use]
    fn with_inner_mut<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&mut O) -> T,
    {
        self.write().unwrap().as_mut().map(f)
    }
}
