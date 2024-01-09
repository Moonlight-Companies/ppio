use futures::Future;

use crate::chan::{unbounded, Receiver, Sender};
use crate::pollers::{IntoPoller, Poller};
use crate::pushers::EmptyPusher;

// bikeshed
pub type PollOutput = anyhow::Result<std::convert::Infallible>;
pub type PushOutput = anyhow::Result<()>;

pub trait State {
    type State;

    fn update(&mut self, state: Self::State);
}

pub trait Poll {
    /// The type that you are sending to the rest of the app
    type Item: Send;

    /// Poll function
    fn poll(&mut self, tx: Sender<Self::Item>) -> impl Future<Output = PollOutput> + Send + '_;
}

pub fn poll<P: Poll>(p: impl IntoPoller<P>) -> (Poller<P>, Receiver<P::Item>) {
    let (tx, rx) = unbounded();

    (Poller::new(p.into(), tx), rx)
}

pub trait Push {
    type Item: Send;

    fn push(&mut self, item: Self::Item) -> impl Future<Output = PushOutput> + Send + '_;
}

impl<T: Send, U> Push for fn(T) -> U {
    type Item = T;

    async fn push(&mut self, item: Self::Item) -> PushOutput {
        self(item);

        Ok(())
    }
}

pub fn push<T>(rx: Receiver<T>) -> (EmptyPusher, Receiver<T>) {
    (EmptyPusher, rx)
}
