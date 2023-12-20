// // use ppio::{Poll, State};

// use std::future::IntoFuture;

// use ppio::prelude::*;
// use ppio::Client;

// struct Channels;

// struct Redis(Client, Option<Channels>);

// impl IntoPoller<Redis> for Client {
//     fn into(self) -> Redis {
//         todo!()
//     }
// }

// impl State for Redis {
//     type State = Channels;

//     fn update(&mut self, state: Self::State) {
//         self.1.replace(state);
//     }
// }

// #[async_trait::async_trait]
// impl Poll for Redis {
//     type Item = ();

//     async fn poll(&mut self, tx: Sender<Self::Item>) -> PollOutput {
//         todo!()
//     }
// }

// #[tokio::main]
// async fn main() {
//     // Pipe<Receiver<T>, M> // (local  map)
//     // Pipe<Poller<T>, M>   // (poller map)

//     // let (poller, [rx]) = poll(Client).with_state();

//     tokio::spawn(poller.into_future());

//     while let Ok(ok) = rx.recv().await {}
// }
fn main() {}
