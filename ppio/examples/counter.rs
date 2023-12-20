use std::future::IntoFuture;

use async_trait::async_trait;
use ppio::prelude::*;

#[derive(Default)]
pub struct Counter;

#[async_trait]
impl Poll for Counter {
    type Item = usize;

    async fn poll(&mut self, tx: Sender<Self::Item>) -> PollOutput {
        let mut count = 0;
        let mut timer = tokio::time::interval(tokio::time::Duration::from_secs(1));

        loop {
            timer.tick().await;
            tx.send(count).await?;
            count += 1;
        }
    }
}

#[tokio::main]
async fn main() {
    let (ctr, rx) = poll(Counter);
    let dbg = push(rx).to_fn(|d| println!("{d:?}"));

    // multithread
    // let _ = tokio::join!(
    //     tokio::spawn(ctr.into_future()),
    //     tokio::spawn(dbg.into_future())
    // );

    // single thread
    let _ = tokio::join!(ctr.into_future(), dbg.into_future());
}
