use std::convert::Infallible;
use std::future::IntoFuture;
use std::{future::Future, pin::Pin, task, task::Poll::*};

use futures::future::BoxFuture;
use futures::Stream;
use pin_project_lite::pin_project;

use crate::channel::{Receiver, RecvError};
use crate::Error::*;
use crate::io::Push;
use crate::util::as_static_mut;

pub struct Pusher<T, P: Push<T>> {
    recver: Receiver<T>,
    pusher: P,
}

impl<T, P: Push<T>> Pusher<T, P> {
    pub fn new(pusher: P, rx: Receiver<T>) -> Self {
        Self { recver: rx, pusher }
    }

    pub fn take_parts(self) -> (Receiver<T>, P) {
        (self.recver, self.pusher)
    }
}

impl<T, P: Push<T> + 'static> IntoFuture for Pusher<T, P> {
    type Output = Result<Infallible, crate::Error>;
    type IntoFuture = Fut<T, P>;

    fn into_future(self) -> Self::IntoFuture {
        Fut {
            fut: None,
            recver: self.recver,
            pusher: self.pusher,
        }
    }
}

pin_project! {
    pub struct Fut<T, P: Push<T>> {
        #[pin]
        fut: Option<BoxFuture<'static, anyhow::Result<()>>>,
        #[pin]
        recver: Receiver<T>,
        pusher: P,
    }
}

impl<T, P: Push<T> + 'static> Future for Fut<T, P> {
    type Output = Result<Infallible, crate::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let mut proj = self.project();

        if let Some(fut) = proj.fut.as_mut().as_pin_mut() {
            futures::ready!(fut.poll(cx).map_err(User)?);
            proj.fut.set(None);
        }

        if let Some(item) = futures::ready!(proj.recver.poll_next(cx)) {
            let pusher = unsafe { as_static_mut(proj.pusher) };

            proj.fut.set(Some(Box::pin(pusher.push(item))));
            cx.waker().wake_by_ref();
            Pending
        } else {
            Ready(Err(Internal(RecvError.into())))
        }
    }
}
