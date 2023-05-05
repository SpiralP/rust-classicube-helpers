use crate::WithBorrow;

pub trait WithInner<O> {
    #[must_use]
    fn with_inner<F, R>(&'static self, f: F) -> Option<R>
    where
        F: FnOnce(&O) -> R;

    #[must_use]
    fn with_inner_mut<F, R>(&'static self, f: F) -> Option<R>
    where
        F: FnOnce(&mut O) -> R;
}

impl<'a, S, O> WithInner<O> for S
where
    S: WithBorrow<'a, Option<O>>,
{
    fn with_inner<F, R>(&'a self, f: F) -> Option<R>
    where
        F: FnOnce(&O) -> R,
    {
        self.with_borrow(|o| o.as_ref().map(f))
    }

    fn with_inner_mut<F, R>(&'a self, f: F) -> Option<R>
    where
        F: FnOnce(&mut O) -> R,
    {
        self.with_borrow_mut(|o| o.as_mut().map(f))
    }
}

#[test]
fn test_with_inner_thread_local() {
    use std::cell::RefCell;

    thread_local!(
        static THREAD_LOCAL: RefCell<Option<u8>> = RefCell::default();
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
    use std::sync::Mutex;

    use lazy_static::lazy_static;

    lazy_static! {
        static ref STATIC_MUTEX: Mutex<Option<u8>> = Mutex::default();
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
    use std::sync::RwLock;

    use lazy_static::lazy_static;

    lazy_static! {
        static ref STATIC_RWLOCK: RwLock<Option<u8>> = RwLock::default();
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
