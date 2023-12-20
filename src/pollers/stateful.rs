use std::future::IntoFuture;
use std::{future::Future, pin::Pin, task};

use futures::future::BoxFuture;
use pin_project_lite::pin_project;

use crate::chan::{Receiver, Sender};
use crate::deferred::{Defer, Undefer, UndeferFuture};
use crate::io::{Poll, PollOutput, State};

pin_project! {
    pub struct Poller<P: Poll>
    where
        P: State
    {
        poller: P,
        srx: Option<Receiver<P::State>>,
        tx: Sender<P::Item>,
    }
}

impl<P: Poll + State> Poller<P> {
    pub(super) fn new(poller: P, tx: Sender<P::Item>, srx: Receiver<P::State>) -> Self {
        Self {
            poller,
            srx: Some(srx),
            tx,
        }
    }
}

impl<P: Poll + State + 'static> IntoFuture for Poller<P> {
    type Output = PollOutput;
    type IntoFuture = UndeferFuture<'static, Poller<P>>;

    fn into_future(self) -> Self::IntoFuture {
        UndeferFuture::new(self)
    }
}

pin_project! {
    pub struct Fut<'a, T> {
        #[pin]
        state_rx: Option<kanal::ReceiveFuture<'a, T>>,
        #[pin]
        inner: BoxFuture<'a, PollOutput>
    }
}

impl<'a, T> Future for Fut<'a, T> {
    type Output = PollOutput;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        self.project().inner.poll(cx)
    }
}

impl<'a, P: Poll + State + 'a> Defer<'a> for Poller<P> {
    type Future = Fut<'a, P::State>;

    fn into_fut(self: Pin<&'a mut Self>) -> Self::Future {
        let proj = self.project();

        Fut {
            state_rx: proj.srx.as_ref().map(|s| s.recv()),
            inner: proj.poller.poll(proj.tx.clone()),
        }
    }
}

impl<'a, P: Poll + State + 'a> Undefer<'a> for Poller<P> {
    type State = Result<P::State, kanal::ReceiveError>;

    fn poll_undefer(
        fut: Pin<&mut Self::Future>,
        cx: &mut task::Context<'_>,
    ) -> Option<Self::State> {
        match fut.project().state_rx.as_pin_mut()?.poll(cx) {
            task::Poll::Ready(item) => Some(item),
            _ => None,
        }
    }

    fn update(self: Pin<&mut Self>, state: Self::State) {
        let proj = self.project();

        match state {
            Ok(inc) => proj.poller.update(inc),
            Err(_) => *proj.srx = None,
        }
    }
}
