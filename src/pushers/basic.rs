use std::convert::Infallible;
use std::future::IntoFuture;
use std::{future::Future, pin::Pin, task, task::Poll::*};

use futures::future::BoxFuture;
use futures::Stream;
use pin_project_lite::pin_project;

use crate::channel::{Receiver, RecvError};
use crate::io::{PollOutput, Push, PushOutput};
use crate::util::as_static_mut;

pub struct Pusher<P: Push> {
    rx: Receiver<P::Item>,
    pusher: P,
}

impl<P: Push> Pusher<P> {
    pub fn new(pusher: P, rx: Receiver<P::Item>) -> Self {
        Self { rx, pusher }
    }
}

impl<P: Push + 'static> IntoFuture for Pusher<P> {
    type Output = PollOutput;
    type IntoFuture = Fut<P>;

    fn into_future(self) -> Self::IntoFuture {
        Fut {
            fut: None,
            recver: self.rx,
            pusher: self.pusher,
        }
    }
}

pin_project! {
    pub struct Fut<P: Push> {
        #[pin]
        fut: Option<BoxFuture<'static, PushOutput>>,
        #[pin]
        recver: Receiver<P::Item>,
        pusher: P,
    }
}

impl<P: Push + 'static> Future for Fut<P> {
    type Output = Result<Infallible, anyhow::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let mut proj = self.project();

        if let Some(fut) = proj.fut.as_mut().as_pin_mut() {
            futures::ready!(fut.poll(cx)?)
        }

        if let Some(item) = futures::ready!(proj.recver.poll_next(cx)) {
            let pusher = unsafe { as_static_mut(proj.pusher) };

            proj.fut.set(Some(Box::pin(pusher.push(item))));
            cx.waker().wake_by_ref();
            Pending
        } else {
            Ready(Err(RecvError.into()))
        }
    }
}
