use std::convert::Infallible;
use std::future::IntoFuture;
use std::{future::Future, pin::Pin, task, task::Poll::*};

use futures::future::BoxFuture;
use futures::Stream;
use pin_project_lite::pin_project;

use crate::channel::{Receiver, RecvError};
use crate::io::{PollOutput, Push, PushOutput, State};
use crate::util::as_static_mut;

pub struct Pusher<T, S, P> {
    recver: Receiver<T>,
    state_recver: Receiver<S>,
    pusher: P,
}

impl<T, S, P> Pusher<T, S, P> {
    pub fn new(pusher: P, rx: Receiver<T>, srx: Receiver<S>) -> Self {
        Self { recver: rx, state_recver: srx, pusher }
    }
}

impl<T, S, P: Push<T> + State<S> + 'static> IntoFuture for Pusher<T, S, P> {
    type Output = PollOutput;
    type IntoFuture = Fut<T, S, P>;

    fn into_future(self) -> Self::IntoFuture {
        Fut {
            fut: None,
            recver: self.recver,
            state_recver: self.state_recver,
            pusher: self.pusher,
        }
    }
}

pin_project! {
    pub struct Fut<T, S, P: Push<T>>
    where
        P: State<S>,
    {
        #[pin]
        fut: Option<BoxFuture<'static, PushOutput>>,
        #[pin]
        recver: Receiver<T>,
        #[pin]
        state_recver: Receiver<S>,
        pusher: P,
    }
}

impl<T, S, P: Push<T> + State<S> + 'static> Future for Fut<T, S, P> {
    type Output = Result<Infallible, anyhow::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let mut proj = self.project();

        if let Some(fut) = proj.fut.as_mut().as_pin_mut() {
            futures::ready!(fut.poll(cx)?);
            proj.fut.set(None);
        }

        loop {
            match proj.state_recver.as_mut().poll_next(cx) {
                Ready(Some(state)) => {
                    proj.pusher.update(state);
                }
                Ready(None) => return Ready(Err(RecvError.into())),
                Pending => break,
            }
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
