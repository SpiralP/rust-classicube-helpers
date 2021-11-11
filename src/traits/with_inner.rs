use crate::WithBorrow;

pub trait WithInner<O> {
    fn with_inner<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&O) -> T;

    fn with_inner_mut<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&mut O) -> T;
}

impl<S, O> WithInner<O> for S
where
    S: WithBorrow<Option<O>>,
{
    fn with_inner<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&O) -> T,
    {
        self.with_borrow(|o| o.as_ref().map(f))
    }

    fn with_inner_mut<F, T>(&'static self, f: F) -> Option<T>
    where
        F: FnOnce(&mut O) -> T,
    {
        self.with_borrow_mut(|o| o.as_mut().map(f))
    }
}

#[test]
fn test_with_inner_thread_local() {
    thread_local!(
        static THREAD_LOCAL: std::cell::RefCell<Option<u8>> = Default::default();
    );
    assert!(THREAD_LOCAL.with_inner(|o| o + 2).is_none());
    THREAD_LOCAL.with_borrow_mut(|option| {
        *option = Some(2);
    });
    assert_eq!(
        THREAD_LOCAL.with_inner_mut(|o| {
            *o += 2;
            *o
        }),
        Some(4)
    );
    assert_eq!(THREAD_LOCAL.with_inner(|o| o + 2), Some(6));
}

#[test]
fn test_with_inner_static_mutex() {
    lazy_static::lazy_static! {
        static ref STATIC_MUTEX: std::sync::Mutex<Option<u8>> = Default::default();
    };
    assert!(STATIC_MUTEX.with_inner(|o| o + 2).is_none());
    STATIC_MUTEX.with_borrow_mut(|option| {
        *option = Some(2);
    });
    assert_eq!(
        STATIC_MUTEX.with_inner_mut(|o| {
            *o += 2;
            *o
        }),
        Some(4)
    );
    assert_eq!(STATIC_MUTEX.with_inner(|o| o + 2), Some(6));
}

#[test]
fn test_with_inner_static_rwlock() {
    lazy_static::lazy_static! {
        static ref STATIC_RWLOCK: std::sync::RwLock<Option<u8>> = Default::default();
    };
    assert!(STATIC_RWLOCK.with_inner(|o| o + 2).is_none());
    STATIC_RWLOCK.with_borrow_mut(|option| {
        *option = Some(2);
    });
    assert_eq!(
        STATIC_RWLOCK.with_inner_mut(|o| {
            *o += 2;
            *o
        }),
        Some(4)
    );
    assert_eq!(STATIC_RWLOCK.with_inner(|o| o + 2), Some(6));
}
