use std::future::IntoFuture;
use std::{future::Future, pin::Pin, task};

use futures::Stream;
use kanal::{ReceiveError, ReceiveStream};
use pin_project_lite::pin_project;

use crate::chan::Receiver;
use crate::fut::extend;
use crate::prelude::PollOutput;

pub struct Pusher<F: Fn(T), T> {
    rx: Receiver<T>,
    func: F,
}

impl<F: Fn(T), T> Pusher<F, T> {
    pub fn new(func: F, rx: Receiver<T>) -> Self {
        Self { rx, func }
    }
}

impl<F: Fn(T) + 'static, T: 'static> IntoFuture for Pusher<F, T> {
    type Output = PollOutput;
    type IntoFuture = Fut<'static, F, T>;

    fn into_future(self) -> Self::IntoFuture {
        Fut {
            inner: None,
            rx: self.rx,
            func: self.func,
        }
    }
}

pin_project! {
    pub struct Fut<'a, F: Fn(T), T> {
        #[pin]
        inner: Option<ReceiveStream<'a, T>>,
        rx: Receiver<T>,
        func: F,
    }
}

impl<'a, F: Fn(T), T> Future for Fut<'a, F, T> {
    type Output = PollOutput;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let mut proj = self.project();

        let fut = match proj.inner.as_mut().as_pin_mut() {
            Some(fut) => fut,
            None => {
                let fut = unsafe { extend(proj.rx).stream() };
                proj.inner.set(Some(fut));
                proj.inner.as_mut().as_pin_mut().unwrap()
            }
        };

        if let Some(item) = futures::ready!(fut.poll_next(cx)) {
            (proj.func)(item);
            cx.waker().wake_by_ref();
            task::Poll::Pending
        } else {
            task::Poll::Ready(Err(ReceiveError::Closed.into()))
        }
    }
}
