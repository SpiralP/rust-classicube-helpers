use std::{cell::Cell, thread::LocalKey};

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
