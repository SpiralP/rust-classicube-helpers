use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Mutex,
    task::{Context, Poll, Waker},
    time::Duration,
};

use async_dispatcher::{Dispatcher, DispatcherHandle, LocalDispatcherHandle};
use futures::{future::Either, prelude::*};
use futures_timer::Delay;
use lazy_static::lazy_static;
use tokio::task::{JoinError, JoinHandle};
use tracing::{debug, warn, Instrument};

use crate::{tick::TickEventHandler, WithInner};

thread_local!(
    static ASYNC_DISPATCHER: RefCell<Option<Dispatcher>> = RefCell::default();
);

thread_local!(
    static ASYNC_DISPATCHER_LOCAL_HANDLE: RefCell<Option<LocalDispatcherHandle>> =
        RefCell::default();
);

lazy_static! {
    static ref ASYNC_DISPATCHER_HANDLE: Mutex<Option<DispatcherHandle>> = Mutex::default();
}

lazy_static! {
    static ref TOKIO_RUNTIME: Mutex<Option<tokio::runtime::Runtime>> = Mutex::default();
}

thread_local!(
    static TICK_HANDLER: RefCell<Option<TickEventHandler>> = RefCell::default();
);

#[tracing::instrument]
pub fn initialize() {
    debug!("async_manager");

    {
        debug!("async_dispatcher");
        let async_dispatcher = Dispatcher::new();
        let async_dispatcher_handle = async_dispatcher.get_handle();
        ASYNC_DISPATCHER_LOCAL_HANDLE.with(|cell| {
            *cell.borrow_mut() = Some(async_dispatcher.get_handle_local());
        });
        ASYNC_DISPATCHER.with(|cell| {
            *cell.borrow_mut() = Some(async_dispatcher);
        });

        *ASYNC_DISPATCHER_HANDLE.lock().unwrap() = Some(async_dispatcher_handle);
    }
    {
        debug!("tokio");
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        *TOKIO_RUNTIME.lock().unwrap() = Some(rt);
    }

    #[cfg(not(test))]
    {
        debug!("tick_handler");
        TICK_HANDLER.with(|cell| {
            let mut tick_handler = TickEventHandler::new();
            tick_handler.on(|_task| {
                step();
            });

            *cell.borrow_mut() = Some(tick_handler);
        });
    }
}

#[tracing::instrument]
pub fn shutdown() {
    debug!("async_manager");

    {
        let mut option = TOKIO_RUNTIME.lock().unwrap();
        if option.is_some() {
            debug!("tokio");
            if let Some(rt) = option.take() {
                rt.shutdown_timeout(Duration::from_millis(100));
            }
        } else {
            warn!("tokio already shutdown?");
        }
    }

    {
        if ASYNC_DISPATCHER.with_inner(|_| ()).is_some() {
            debug!("async_dispatcher");

            ASYNC_DISPATCHER_HANDLE.lock().unwrap().take();
            ASYNC_DISPATCHER_LOCAL_HANDLE.with(|cell| cell.borrow_mut().take());
            ASYNC_DISPATCHER.with(|cell| cell.borrow_mut().take());
        } else {
            warn!("async_dispatcher already shutdown?");
        }
    }

    #[cfg(not(test))]
    {
        if TICK_HANDLER.with_inner(|_| ()).is_some() {
            debug!("tick_handler");

            TICK_HANDLER.with(|cell| cell.borrow_mut().take());
        } else {
            warn!("tick_handler already shutdown?");
        }
    }
}

#[derive(Default, Clone)]
struct YieldedWaker {
    waker: Rc<Cell<Option<Waker>>>,
    woke: Rc<Cell<bool>>,
}
impl Future for YieldedWaker {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.woke.get() {
            Poll::Ready(())
        } else {
            self.waker.set(Some(cx.waker().clone()));
            Poll::Pending
        }
    }
}

thread_local!(
    static YIELDED_WAKERS: RefCell<Vec<YieldedWaker>> = RefCell::default();
);

pub fn step() {
    YIELDED_WAKERS.with(move |cell| {
        let vec = &mut *cell.borrow_mut();
        for state in vec.drain(..) {
            state.woke.set(true);
            if let Some(waker) = state.waker.take() {
                waker.wake();
            }
        }
    });

    // process futures
    ASYNC_DISPATCHER
        .with_inner_mut(|async_dispatcher| {
            async_dispatcher.run_until_stalled();
        })
        .unwrap();
}

/// Run all tasks in the pool to completion.
pub fn run() {
    ASYNC_DISPATCHER
        .with_inner_mut(|async_dispatcher| {
            async_dispatcher.run();
        })
        .unwrap();
}

pub async fn sleep(duration: Duration) {
    Delay::new(duration).await;
}

pub async fn yield_now() {
    let waker = YieldedWaker::default();
    {
        let waker = waker.clone();
        YIELDED_WAKERS.with(move |ref_cell| {
            let vec = &mut *ref_cell.borrow_mut();
            vec.push(waker);
        });
    }
    waker.await;
}

pub async fn timeout<T, F>(duration: Duration, f: F) -> Option<T>
where
    F: Future<Output = T> + Send,
{
    let delay = Delay::new(duration);

    match future::select(delay, f.boxed()).await {
        Either::Left((_, _f)) => None,
        Either::Right((r, _delay)) => Some(r),
    }
}

pub async fn timeout_local<T, F>(duration: Duration, f: F) -> Option<T>
where
    F: Future<Output = T>,
{
    let delay = Delay::new(duration);

    match future::select(delay, f.boxed_local()).await {
        Either::Left((_, _f)) => None,
        Either::Right((r, _delay)) => Some(r),
    }
}

/// Block thread until future is resolved.
///
/// This will continue to call the same executor so `cef_step()` will still be called!
pub fn block_on_local<F>(f: F)
where
    F: Future<Output = ()> + 'static,
{
    let shared = f.in_current_span().shared();

    {
        let shared = shared.clone();
        spawn_local_on_main_thread(async move {
            shared.await;
        });
    }

    loop {
        match shared.peek() {
            Some(_) => {
                return;
            }

            None => {
                step();
            }
        }

        // don't burn anything
        std::thread::sleep(Duration::from_millis(16));
    }
}

pub fn spawn<F>(f: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    TOKIO_RUNTIME
        .with_inner(move |rt| rt.spawn(f.in_current_span()))
        .unwrap()
}

pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<Result<R, JoinError>>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let span = tracing::Span::current();
    spawn(async move {
        tokio::task::spawn_blocking(move || {
            let _enter = span.enter();
            f()
        })
        .await
    })
}

pub fn spawn_on_main_thread<F>(f: F)
where
    F: Future<Output = ()> + 'static + Send,
{
    let mut handle = {
        let mut handle = ASYNC_DISPATCHER_HANDLE.lock().unwrap();
        handle.as_mut().expect("handle.as_mut()").clone()
    };

    handle.spawn(f.in_current_span());
}

pub async fn run_on_main_thread<F, O>(f: F) -> O
where
    F: Future<Output = O> + 'static + Send,
    O: 'static + Send + std::fmt::Debug,
{
    let mut handle = {
        let mut handle = ASYNC_DISPATCHER_HANDLE.lock().unwrap();
        handle.as_mut().expect("handle.as_mut()").clone()
    };

    handle.dispatch(f.in_current_span()).await
}

pub fn spawn_local_on_main_thread<F>(f: F)
where
    F: Future<Output = ()> + 'static,
{
    let mut handle = ASYNC_DISPATCHER_LOCAL_HANDLE
        .with_inner(Clone::clone)
        .expect("ASYNC_DISPATCHER_LOCAL_HANDLE is None");

    handle.spawn(f.in_current_span());
}

#[test]
fn test_async_manager() {
    use tracing::info;
    use tracing_subscriber::{filter::EnvFilter, prelude::*};

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("debug".parse().unwrap()))
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(true)
        .without_time()
        .finish()
        .init();

    initialize();

    {
        #[tracing::instrument]
        fn test() {
            let a = tracing::info_span!("A");
            let span = a.enter();
            spawn(async move {
                let b = tracing::info_span!("B");
                let span = b.enter();
                run_on_main_thread(async move {
                    let c = tracing::info_span!("C");
                    let span = c.enter();
                    info!("run_on_main_thread with instrument test:A:B:C");
                    drop(span);
                })
                .await;
                drop(span);
            });
            drop(span);
        }
        test();
    }

    {
        #[tracing::instrument]
        fn test() {
            let a = tracing::info_span!("A");
            let span = a.enter();
            spawn_blocking(|| {
                let b = tracing::info_span!("B");
                let span = b.enter();
                info!("spawn_blocking with instrument test:A:B");
                drop(span);
            });
            drop(span);
        }
        test();
    }

    {
        #[tracing::instrument]
        fn test() {
            let a = tracing::info_span!("A");
            let span = a.enter();
            block_on_local(async move {
                let b = tracing::info_span!("B");
                let span = b.enter();
                info!("block_on_local with instrument test:A:B");
                spawn(async move {
                    let c = tracing::info_span!("C");
                    let span = c.enter();
                    info!("block_on_local spawn with instrument test:A:B:C");
                    drop(span);
                });
                drop(span);
            });
            drop(span);
        }
        test();
    }

    let stopped = std::sync::Arc::new(Mutex::new(false));
    {
        let stopped = stopped.clone();
        spawn(async move {
            sleep(Duration::from_secs(1)).await;
            *stopped.lock().unwrap() = true;
        });
    }
    while !*stopped.lock().unwrap() {
        step();
        std::thread::sleep(Duration::from_millis(10));
    }

    shutdown();
}
