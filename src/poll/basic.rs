use std::{future::IntoFuture, pin::Pin};

use async_trait::async_trait;
use futures::future::BoxFuture;
use pin_project_lite::pin_project;

use super::PollOutput;
use crate::deferred::{Defer, DeferFuture};


use crate::chan::AsyncSender;
use crate::chan::Sender;


#[async_trait]
pub trait Poll {
    /// item that you are polling
    type Item: Send;

    /// Poll function
    async fn poll(&mut self, tx: impl Sender<Self::Item>) -> PollOutput;
}

pin_project! {
    pub struct Poller<P: Poll, S: Sender<P::Item> = AsyncSender<<P as Poll>::Item>> {
        poller: P,
        tx: Option<S>,
    }
}

impl<P: Poll, S: Sender<P::Item>> Poller<P, S> {
    pub(super) fn new(poller: P, tx: S) -> Self {
        Self {
            poller,
            tx: Some(tx),
        }
    }

    pub(super) fn take_poller(self) -> P {
        self.poller
    }
}

impl<P: Poll + 'static, S: Sender<P::Item> + 'static> IntoFuture for Poller<P, S> {
    type Output = PollOutput;
    type IntoFuture = DeferFuture<'static, Self>;

    fn into_future(self) -> Self::IntoFuture {
        DeferFuture::new(self)
    }
}

impl<'a, P: Poll + 'a, S: Sender<P::Item> + 'static> Defer<'a> for Poller<P, S> {
    type Future = BoxFuture<'a, PollOutput>;

    fn into_fut(self: Pin<&'a mut Self>) -> Self::Future {
        let proj = self.project();

        if proj.tx.is_none() {
            panic!("this poller is partially initialized!")
        }

        proj.poller.poll(proj.tx.take().unwrap())
    }
}
