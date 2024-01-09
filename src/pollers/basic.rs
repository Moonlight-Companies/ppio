use std::{future::IntoFuture, pin::Pin};

use futures::future::BoxFuture;
use pin_project_lite::pin_project;

use crate::chan::Sender;
use crate::deferred::{Defer, DeferFuture};
use crate::io::{Poll, PollOutput};

pin_project! {
    pub struct Poller<P: Poll> {
        poller: P,
        tx: Sender<P::Item>,
    }
}

impl<P: Poll> Poller<P> {
    pub(crate) fn new(poller: P, tx: Sender<P::Item>) -> Self {
        Self { poller, tx }
    }

    pub(crate) fn take_poller(self) -> P {
        self.poller
    }
}

impl<P: Poll + 'static> IntoFuture for Poller<P> {
    type Output = PollOutput;
    type IntoFuture = DeferFuture<'static, Self>;

    fn into_future(self) -> Self::IntoFuture {
        DeferFuture::new(self)
    }
}

impl<'a, P: Poll + 'a> Defer<'a> for Poller<P> {
    type Future = BoxFuture<'a, PollOutput>;

    fn into_fut(self: Pin<&'a mut Self>) -> Self::Future {
        let proj = self.project();

        Box::pin(proj.poller.poll(proj.tx.clone()))
    }
}
