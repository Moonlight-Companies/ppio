use std::convert::Infallible;
use std::future::IntoFuture;
use std::{future::Future, pin::Pin, task};

use futures::future::BoxFuture;
use pin_project_lite::pin_project;

use crate::channel::Sender;
use crate::io::Poll;
use crate::Error::*;
use crate::util::as_static_mut;

pub struct Poller<P: Poll> {
    poller: P,
    sender: Sender<P::Item>,
}

impl<P: Poll> Poller<P> {
    pub(crate) fn new(poller: P, tx: Sender<P::Item>) -> Self {
        Self { poller, sender: tx }
    }

    pub(crate) fn take_poller(self) -> P {
        self.poller
    }
}

impl<P: Poll + 'static> IntoFuture for Poller<P> {
    type Output = Result<Infallible, crate::Error>;
    type IntoFuture = Fut<P>;

    fn into_future(self) -> Self::IntoFuture {
        Fut {
            fut: None,
            poller: self.poller,
            sender: self.sender,
        }
    }
}

pin_project! {
    pub struct Fut<P: Poll> {
        #[pin]
        fut: Option<BoxFuture<'static, anyhow::Result<Infallible>>>,
        poller: P,
        sender: Sender<P::Item>
    }
}

impl<P: Poll + 'static> Future for Fut<P> {
    type Output = Result<Infallible, crate::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let mut proj = self.project();

        if proj.fut.is_none() {
            let poller = unsafe { as_static_mut(proj.poller) };
            let fut = poller.poll(proj.sender.clone());

            proj.fut.set(Some(Box::pin(fut)));
        }

        proj.fut.as_pin_mut().unwrap().poll(cx).map_err(User)
    }
}
