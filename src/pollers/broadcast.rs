use std::convert::Infallible;
use std::future::IntoFuture;
use std::{future::Future, pin::Pin, task, task::Poll::*};

use futures::future::BoxFuture;
use futures::Stream;
use pin_project_lite::pin_project;

use crate::channel::{bounded, Receiver, SendError, Sender};
use crate::io::{Poll, PollOutput};
use crate::util::as_static_mut;

pub struct Poller<P: Poll> {
    poller: P,
    senders: Vec<Sender<P::Item>>,
}

impl<P: Poll> Poller<P> {
    pub(crate) fn new(poller: P, txs: Vec<Sender<P::Item>>) -> Self {
        Self {
            poller,
            senders: txs,
        }
    }
}

impl<P: Poll + 'static> IntoFuture for Poller<P>
where
    P::Item: Clone,
{
    type Output = PollOutput;
    type IntoFuture = Fut<P>;

    fn into_future(self) -> Self::IntoFuture {
        Fut {
            fut: None,
            recver: None,
            poller: self.poller,
            senders: self.senders,
        }
    }
}

pin_project! {
    pub struct Fut<P: Poll> {
        #[pin]
        fut: Option<BoxFuture<'static, PollOutput>>,
        #[pin]
        recver: Option<Receiver<P::Item>>,
        poller: P,
        senders: Vec<Sender<P::Item>>
    }
}

impl<P: Poll + 'static> Future for Fut<P>
where
    P::Item: Clone,
{
    type Output = Result<Infallible, anyhow::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let mut proj = self.project();

        if proj.fut.is_none() && proj.recver.is_none() {
            let poller = unsafe { as_static_mut(proj.poller) };
            let (tx, rx) = bounded(1);
            let fut = poller.poll(tx);

            proj.fut.set(Some(Box::pin(fut)));
            proj.recver.set(Some(rx));
        }

        let fut = proj.fut.as_pin_mut().unwrap();
        let recver = proj.recver.as_pin_mut().unwrap();

        // the future is always pending after this point
        let _ = fut.poll(cx)?;

        if let Some(item) = futures::ready!(recver.poll_next(cx)) {
            proj.senders
                .retain_mut(|sender| sender.try_send(item.clone()).is_ok());

            if proj.senders.is_empty() {
                return Ready(Err(SendError(()).into()));
            }
        }

        Pending
    }
}
