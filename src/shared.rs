use std::{
    cell::RefCell,
    ops::DerefMut,
    rc::Rc,
    sync::{Arc, Mutex},
};

use futures::lock::Mutex as FutureMutex;

pub struct SyncShared<T: ?Sized> {
    inner: Rc<RefCell<T>>,
}

impl<T> SyncShared<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(value)),
        }
    }
}

impl<T: ?Sized> SyncShared<T> {
    pub fn lock(&mut self) -> impl DerefMut<Target = T> + '_ {
        self.inner.borrow_mut()
    }

    pub fn with<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.lock();
        f(&mut guard)
    }
}

impl<T: ?Sized> Clone for SyncShared<T> {
    fn clone(&self) -> SyncShared<T> {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct ThreadShared<T: ?Sized> {
    inner: Arc<Mutex<T>>,
}

impl<T> ThreadShared<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
        }
    }

    pub fn lock(&mut self) -> impl DerefMut<Target = T> + '_ {
        self.inner.lock().unwrap()
    }

    pub fn with<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.lock();
        f(&mut guard)
    }
}

impl<T: ?Sized> Clone for ThreadShared<T> {
    fn clone(&self) -> ThreadShared<T> {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct FutureShared<T: ?Sized> {
    inner: Arc<FutureMutex<T>>,
}

impl<T> FutureShared<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(FutureMutex::new(value)),
        }
    }

    pub async fn lock(&mut self) -> impl DerefMut<Target = T> + '_ {
        self.inner.lock().await
    }

    pub async fn with<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.lock().await;
        f(&mut guard)
    }

    pub async fn with_async<F, FUT, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> FUT,
        FUT: futures::Future<Output = R>,
    {
        let mut guard = self.lock().await;
        f(&mut guard).await
    }
}

impl<T: ?Sized> Clone for FutureShared<T> {
    fn clone(&self) -> FutureShared<T> {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[test]
fn test_shared() {
    {
        let mut shared = SyncShared::new(1);
        shared.with(|v| {
            println!("{}", v);
        });
        let v = shared.lock();
        println!("{}", {
            let a: &u8 = &v;
            a
        });
    }

    {
        #[derive(Debug)]
        struct NotClone {}

        let mut shared = ThreadShared::new(NotClone {});

        {
            let mut shared = shared.clone();
            std::thread::spawn(move || {
                shared.with(|v| {
                    println!("{:?}", v);
                });
                let v = shared.lock();
                println!("{:?}", {
                    let a: &NotClone = &v;
                    a
                });
            });
        }

        shared.with(|v| {
            println!("{:?}", v);
        });
        let v = shared.lock();
        println!("{:?}", {
            let a: &NotClone = &v;
            a
        });
    }

    futures::executor::block_on(async {
        let mut shared = FutureShared::new(3);
        shared
            .with(|v| {
                println!("{}", v);
            })
            .await;
        let v = shared.lock().await;
        println!("{}", {
            let a: &u8 = &v;
            a
        });
    });

    trait Module {
        fn load(&mut self);
        fn unload(&mut self);
    }

    struct ModuleThing {}
    impl Module for ModuleThing {
        fn load(&mut self) {}

        fn unload(&mut self) {}
    }

    let mut list_of_sync_shareds: Vec<Rc<Box<dyn Module>>> = Vec::new();
    let mod_thing: Rc<Box<dyn Module>> = Rc::new(Box::new(ModuleThing {}));
    list_of_sync_shareds.push(mod_thing);

    let mut list_of_sync_shareds: Vec<SyncShared<Box<dyn Module>>> = Vec::new();
    let mod_thing: SyncShared<Box<dyn Module>> = SyncShared::new(Box::new(ModuleThing {}));
    list_of_sync_shareds.push(mod_thing);

    for module in list_of_sync_shareds.iter_mut() {
        module.lock().load();
    }
}
