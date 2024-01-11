use std::convert::Infallible;
use std::future::IntoFuture;
use std::{future::Future, pin::Pin, task, task::Poll::*};

use futures::future::BoxFuture;
use futures::Stream;
use pin_project_lite::pin_project;

use crate::channel::{Receiver, Sender, RecvError};
use crate::io::{Poll, PollOutput, State};
use crate::util::as_static_mut;

pub struct Poller<S, P: Poll + State<S>> {
    poller: P,
    recver: Receiver<S>,
    sender: Sender<P::Item>,
}

impl<S, P: Poll + State<S>> Poller<S, P> {
    pub(super) fn new(poller: P, tx: Sender<P::Item>, srx: Receiver<S>) -> Self {
        Self {
            poller,
            recver: srx,
            sender: tx,
        }
    }
}

impl<S, P: Poll + State<S> + 'static> IntoFuture for Poller<S, P> {
    type Output = PollOutput;
    type IntoFuture = Fut<S, P>;

    fn into_future(self) -> Self::IntoFuture {
        Fut {
            fut: None,
            recver: self.recver,
            poller: self.poller,
            sender: self.sender,
        }
    }
}

pin_project! {
    pub struct Fut<S, P: Poll>
    where
        P: State<S>,
    {
        #[pin]
        fut: Option<BoxFuture<'static, PollOutput>>,
        #[pin]
        recver: Receiver<S>,
        poller: P,
        sender: Sender<P::Item>
    }
}

impl<S, P: Poll + State<S> + 'static> Future for Fut<S, P> {
    type Output = Result<Infallible, anyhow::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let mut proj = self.project();

        match proj.recver.poll_next(cx) {
            Ready(Some(state)) => {
                proj.fut.set(None);
                proj.poller.update(state);
            },
            Ready(None) => return Ready(Err(RecvError.into())),
            _ => (),
        }

        if proj.fut.is_none() {
            let poller = unsafe { as_static_mut(proj.poller) };
            let fut = poller.poll(proj.sender.clone());

            proj.fut.set(Some(Box::pin(fut)));
        }

        proj.fut.as_pin_mut().unwrap().poll(cx)
    }
}
