use std::convert::Infallible;
use std::future::IntoFuture;
use std::{future::Future, pin::Pin, task, task::Poll::*};

use futures::future::BoxFuture;
use futures::Stream;
use pin_project_lite::pin_project;

use crate::channel::{Receiver, Sender};
use crate::io::{Poll, PollOutput, State};
use crate::util::as_static_mut;

pub struct Poller<P: Poll + State> {
    poller: P,
    recver: Receiver<P::State>,
    sender: Sender<P::Item>,
}

impl<P: Poll + State> Poller<P> {
    pub(super) fn new(poller: P, tx: Sender<P::Item>, srx: Receiver<P::State>) -> Self {
        Self {
            poller,
            recver: srx,
            sender: tx,
        }
    }
}

impl<P: Poll + State + 'static> IntoFuture for Poller<P> {
    type Output = PollOutput;
    type IntoFuture = Fut<P>;

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
    pub struct Fut<P: Poll>
    where
        P: State,
    {
        #[pin]
        fut: Option<BoxFuture<'static, PollOutput>>,
        #[pin]
        recver: Receiver<P::State>,
        poller: P,
        sender: Sender<P::Item>
    }
}

impl<P: Poll + State + 'static> Future for Fut<P> {
    type Output = Result<Infallible, anyhow::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let mut proj = self.project();

        match proj.recver.poll_next(cx) {
            Ready(Some(state)) => {
                proj.fut.set(None);
                proj.poller.update(state);
            },
            Ready(None) => todo!(),
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
