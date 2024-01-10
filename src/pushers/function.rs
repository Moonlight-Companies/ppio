use std::future::IntoFuture;
use std::{future::Future, pin::Pin, task, task::Poll::*};

use futures::Stream;
use pin_project_lite::pin_project;

use crate::chan::{Receiver, RecvError};
use crate::prelude::PollOutput;

pub struct Pusher<T, F: Fn(T)> {
    recver: Receiver<T>,
    func: F,
}

impl<T, F: Fn(T)> Pusher<T, F> {
    pub fn new(func: F, rx: Receiver<T>) -> Self {
        Self { recver: rx, func }
    }
}

impl<T, F: Fn(T)> IntoFuture for Pusher<T, F> {
    type Output = PollOutput;
    type IntoFuture = Fut<T, F>;

    fn into_future(self) -> Self::IntoFuture {
        Fut {
            rx: self.recver,
            func: self.func,
        }
    }
}

pin_project! {
    pub struct Fut<T, F: Fn(T)> {
        #[pin]
        rx: Receiver<T>,
        func: F,
    }
}

impl<T, F: Fn(T)> Future for Fut<T, F> {
    type Output = PollOutput;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let proj = self.project();

        if let Some(item) = futures::ready!(proj.rx.poll_next(cx)) {
            (proj.func)(item);
            cx.waker().wake_by_ref();
            Pending
        } else {
            Ready(Err(RecvError.into()))
        }
    }
}
