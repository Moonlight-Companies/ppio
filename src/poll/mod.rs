use futures::future::BoxFuture;

pub type PollOutput = anyhow::Result<std::convert::Infallible>;

pub mod basic;

// pub mod stateful;

// use crate::{unbounded, MapFn, Receiver};

use crate::chan::{AsyncReceiver, unbounded};


pub use basic::{Poll, Poller};
// pub use stateful::{Pair as StatefulPair, Poll as StatefulPoll, Poller as StatefulPoller};




pub fn poll<P: Poll>(p: P) -> (Poller<P>, AsyncReceiver<P::Item>) {
    let (tx, rx) = unbounded();

    (basic::Poller::new(p, tx), rx)
}

pub trait PollerExt<P: Poll> {
    type Poller;

    // fn with_state<S>(self, rx: Receiver<P::State, S>) -> StatefulPair<P, S>
    // where
    //     P: StatefulPoll;

    // fn map_recv<O>(self, f: impl MapFn<P::Item, O>) -> (Self::Poller, Receiver<O, P::Item>);
}

impl<P: Poll> PollerExt<P> for () {
    type Poller = Poller<P>;

    // fn with_state<S>(self, srx: Receiver<P::State, S>) -> StatefulPair<P, S>
    // where
    //     P: StatefulPoll,
    // {
    //     let (tx, rx) = unbounded();

    //     let poller = StatefulPoller::new(self.0.take_poller(), tx, srx);

    //     (poller, rx)
    // }

    // fn map_recv<O>(self, f: impl MapFn<P::Item, O>) -> (Self::Poller, Receiver<O, P::Item>) {
    //     (self.0, self.1.map(f))
    // }
}

// impl<P: StatefulPoll, R> PollerExt<P> for StatefulPair<P, R> {
//     type Poller = StatefulPoller<P, R>;

//     // fn with_state<S>(self, srx: Receiver<P::State, S>) -> StatefulPair<P, S>
//     // where
//     //     P: StatefulPoll,
//     // {
//     //     let (tx, rx) = unbounded();

//     //     let poller = StatefulPoller::new(self.0.take_poller(), tx, srx);

//     //     (poller, rx)
//     // }

//     // fn map_recv<O>(self, f: impl MapFn<P::Item, O>) -> (Self::Poller, Receiver<O, P::Item>) {
//     //     (self.0, self.1.map(f))
//     // }
// }
