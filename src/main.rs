mod deferred;

mod channel;

mod poll;

mod state {
    pub trait State {
        type State: Default + Send;

        fn update(new: Self::State, old: &mut Self::State) {
            *old = new;
        }
    }
}

mod chan {
    use std::future::Future;

    pub use kanal::AsyncReceiver;
    pub use kanal::AsyncSender;
    pub use kanal::bounded_async as bounded;
    pub use kanal::unbounded_async as unbounded;

    use kanal::{ReceiveFuture, SendFuture, SendError};

    use crate::deferred::UndeferFuture;

    use self::bc_future::FutInner;

    pub trait Sender<T>: Send {
        type Future<'a>: Future<Output = Result<(), SendError>> + Send + 'a where Self: 'a;
        type Receiver;

        fn send<'a>(&'a mut self, item: T) -> Self::Future<'a> where Self: 'a;
    }

    impl<T: Send> Sender<T> for AsyncSender<T> {
        type Future<'a> = SendFuture<'a, T> where Self: 'a;
        type Receiver = AsyncReceiver<T>;

        fn send<'a>(&'a mut self, item: T) -> Self::Future<'a> where Self: 'a {
            Self::send(self, item)
        }
    }

    impl<T: Clone + Send> Sender<T> for BroadcastSender<T> {
        type Future<'a> = UndeferFuture<'a, FutInner<'a, T>> where Self: 'a;
        type Receiver = BroadcastReceiver<T>;

        fn send<'a>(&'a mut self, item: T) -> Self::Future<'a> where Self: 'a {
            Self::send(self, item)
        }
    }



    mod bc_future {
        use std::{future::Future, pin::Pin, task};

        use futures::future::{JoinAll, Ready};
        use kanal::{SendFuture, SendError};
        use pin_project_lite::pin_project;

        use crate::deferred::{Defer, Undefer};

        use super::BroadcastSender;


        pin_project! {
            pub struct FutInner<'a, T> {
                bs: &'a mut BroadcastSender<T>,
                itm: T,
                res:  Option<Vec<Result<(), SendError>>>,
            }
        }

        impl<'a, T> FutInner<'a, T> {
            pub fn new(bs: &'a mut BroadcastSender<T>, itm: T) -> Self {
                Self {
                    bs,
                    itm,
                    res: None,
                }
            }
        }
        
        pin_project! {
            #[project = EnumProj]
            enum BroadcastFuture<'a, T> {
                Join {
                    res: Option<Vec<Result<(), SendError>>>,
                    #[pin]
                    fut: JoinAll<SendFuture<'a, T>>
                },
                Ready {
                    #[pin]
                    fut: Ready<Result<(), SendError>>
                }
            }
        }
        
        impl<'a, T: Clone> Future for BroadcastFuture<'a, T> {
            type Output = Result<(), SendError>;

            fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
                match self.project() {
                    EnumProj::Join { res, fut } => match fut.poll(cx) {
                        task::Poll::Ready(ready) => {
                            *res = Some(ready);
                            task::Poll::Pending
                        },
                        _ => task::Poll::Pending,
                    },
                    EnumProj::Ready { fut } => fut.poll(cx),
                }
            }
        }

        impl<'a, T: Clone> Undefer<'a> for FutInner<'a, T> {
            type State = Vec<Result<(), SendError>>;

            fn undefer(fut: Pin<&mut Self::Future>) -> Option<Self::State> {
                match fut.project() {
                    EnumProj::Join { res, .. } => res.take(),
                    EnumProj::Ready { .. } => None,
                }
            }

            fn update(self: Pin<&mut Self>, state: Self::State) {
                let proj = self.project();

                proj.res.replace(state);
            }
        }

        impl<'a, T: Clone> Defer<'a> for FutInner<'a, T> {
            type Future = BroadcastFuture<'a, T>;

            fn into_fut(self: Pin<&'a mut Self>) -> Self::Future {
                let proj = self.project();
                
                match proj.res.take() {
                    Some(res) => {
                        let mut tres: Result<(), SendError> = Err(SendError::ReceiveClosed);

                        for (res, i) in res.into_iter().zip(0..) {
                            if res.is_err() {
                                proj.bs.0.remove(i);
                            } else {
                                tres = Ok(());
                            }
                        }

                        BroadcastFuture::Ready {
                            fut: futures::future::ready(tres)
                        }
                    },
                    None => {
                        let futs = proj.bs
                            .0
                            .iter_mut()
                            .map(|tx| tx.send(proj.itm.clone()));

                        BroadcastFuture::Join {
                            res: None,
                            fut: futures::future::join_all(futs)
                        }
                    }
                }
            }
        }
    }

    pub struct BroadcastSender<T>(Vec<AsyncSender<T>>, AsyncReceiver<AsyncSender<T>>);

    impl<T: Clone> BroadcastSender<T> {
        pub fn send(&mut self, item: T) -> UndeferFuture<FutInner<T>> {
            UndeferFuture::new(FutInner::new(self, item))
        }

        pub fn subscribe(&mut self) -> AsyncReceiver<T> {
            let (tx, rx) = bounded(1);

            self.0.push(tx);

            rx
        }
    }

    pub struct BroadcastReceiver<T> {
        send: AsyncSender<AsyncSender<T>>,
        recv: AsyncReceiver<T>,
    }

    impl<T> BroadcastReceiver<T> {
        pub fn recv(&self) -> ReceiveFuture<'_, T> {
            self.recv.recv()
        }

        pub fn subscribe(&self) -> Result<BroadcastReceiver<T>, SendError> {
            let (tx, rx) = bounded(1);

            self.send.try_send(tx)?;

            Ok(BroadcastReceiver {
                send: self.send.clone(), 
                recv: rx,
            })
        }
    }
}

//////////////////////////////////////////////////////////////

// 10.2.211.110 plc
// 10.2.211.120 hmi

mod prelude {
    pub use crate::chan::*;
    pub use crate::poll::*;
    pub use crate::state::State;
}

use prelude::*;

// deps
use async_trait::async_trait;
struct Helloer(usize);


#[async_trait]
impl Poll for Helloer {
    type Item = String;

    async fn poll(&mut self, mut tx: impl Sender<Self::Item>) -> PollOutput {
        use tokio::time::{sleep, Duration};

        loop {
            tx.send("Hello!".into()).await?;
            self.0 += 1;
            sleep(Duration::from_secs(1)).await;
        }
    }
}



#[tokio::main]
async fn main() {

}








// use std::future::IntoFuture;

// use async_trait::async_trait;



// struct GreetingSource;

// #[async_trait]
// impl Poll for GreetingSource {
//     type Item = usize;

//     async fn poll(&mut self, tx: Sender<Self::Item>) -> PollOutput {
//         use tokio::time::{sleep, Duration};

//         loop {
//             tx.send(0).await?;
//             sleep(Duration::from_secs(5)).await;

//             tx.send(1).await?;
//             sleep(Duration::from_secs(5)).await;

//             tx.send(2).await?;
//             sleep(Duration::from_secs(5)).await;
//         }
//     }
// }

// #[derive(Default)]
// struct Helloer(usize);

// #[derive(Default)]
// struct Greeting(Option<&'static str>);

// impl State for Helloer {
//     type State = Greeting;
// }

// #[async_trait]
// impl StatefulPoll for Helloer {
//     type Item = &'static str;

//     async fn poll(&mut self, state: &mut Self::State, tx: Sender<Self::Item>) -> PollOutput {
//         use tokio::time::{sleep, Duration};

//         if let Some(greeting) = state.0 {
//             loop {
//                 tx.send(greeting).await?;
//                 self.0 += 1;
//                 sleep(Duration::from_secs(1)).await;
//             }
//         }

//         futures::future::pending().await
//     }
// }

// #[tokio::main]
// async fn main() {
//     let hr = Helloer::default();

//     let hello = vec!["Hello!", "Howdy!", "Sup!"];

//     let (pa, rx) = poll(GreetingSource).map_recv(move |i| Greeting(Some(hello[i])));
//     let (pb, rx) = poll(hr).with_state(rx);

//     tokio::spawn(pa.into_future());
//     tokio::spawn(pb.into_future());

//     while let Ok(item) = rx.recv().await {
//         println!("{item}")
//     }
// }
