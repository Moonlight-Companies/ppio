use std::future::IntoFuture;
use std::{future::Future, pin::Pin, task};

use async_trait::async_trait;
use futures::future::BoxFuture;
use futures::Stream;
use pin_project_lite::pin_project;

use super::basic;

use super::PollOutput;
use crate::deferred::{Defer, Undefer, UndeferFuture};
use crate::ReceiveStream;
use crate::State;
use crate::{Receiver, Sender};

#[async_trait]
pub trait Poll: State {
    type Item: Send;

    async fn poll(
        &mut self,
        state: &mut Self::State,
        tx: Sender<<Self as Poll>::Item>,
    ) -> PollOutput;
}

impl<P: Poll> basic::Poll for P {
    type Item = <Self as Poll>::Item;

    fn poll<'a, 't>(&'a mut self, tx: Sender<Self::Item>) -> BoxFuture<'t, PollOutput>
    where
        'a: 't,
        Self: 't,
    {
        let leak = Box::leak(Box::default());

        self.poll(leak, tx)
    }
}

pin_project! {
    pub struct Poller<P: Poll, S = <P as State>::State> {
        poller: P,
        state: P::State,
        srx: Receiver<P::State, S>,
        tx: Sender<P::Item>,
    }
}

pub type Pair<P: Poll, S> = (Poller<P, S>, Receiver<P::Item>);

impl<P: Poll, S> Poller<P, S> {
    pub(super) fn new(poller: P, tx: Sender<P::Item>, srx: Receiver<P::State, S>) -> Self {
        Self {
            poller,
            state: Default::default(),
            srx,
            tx,
        }
    }

    pub(super) fn take_poller(self) -> P {
        self.poller
    }
}

impl<P: Poll + 'static, S: Send + 'static> IntoFuture for Poller<P, S> {
    type Output = PollOutput;
    type IntoFuture = UndeferFuture<'static, Poller<P, S>>;

    fn into_future(self) -> Self::IntoFuture {
        UndeferFuture::new(self)
    }
}

pin_project! {
    pub struct Fut<'a, P: Poll, S> {
        next_state: Option<P::State>,
        #[pin]
        state_stream: ReceiveStream<'a, P::State, S>,
        #[pin]
        inner: BoxFuture<'a, PollOutput>
    }
}

impl<'a, P: Poll + 'a, S: Send + 'a> Defer<'a> for Poller<P, S> {
    type Future = Fut<'a, P, S>;

    fn into_fut(self: Pin<&'a mut Self>) -> Self::Future {
        let proj = self.project();

        Fut {
            next_state: None,
            state_stream: proj.srx.stream(),
            inner: proj.poller.poll(proj.state, proj.tx.clone()),
        }
    }
}

impl<'a, P: Poll + 'a, S: Send + 'a> Undefer<'a> for Poller<P, S> {
    type State = P::State;

    fn undefer(fut: Pin<&mut Self::Future>) -> Option<Self::State> {
        let proj = fut.project();

        proj.next_state.take()
    }

    fn update(self: Pin<&mut Self>, state: Self::State) {
        let proj = self.project();

        <P as State>::update(state, proj.state);
    }
}

impl<'a, P: Poll, S> Future for Fut<'a, P, S> {
    type Output = PollOutput;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let proj = self.project();

        if let task::Poll::Ready(Some(state)) = proj.state_stream.poll_next(cx) {
            proj.next_state.replace(state);

            cx.waker().wake_by_ref();
            return task::Poll::Pending;
        }

        proj.inner.poll(cx)
    }
}
