use ppio::prelude::*;

#[derive(Default)]
pub struct Counter;

impl Poll for Counter {
    type Item = usize;

    async fn poll(&mut self, tx: ppio::channel::Sender<Self::Item>) -> anyhow::Result<Infallible> {
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
    let (counter, rx) = poll(Counter);
    let debug = push(rx).to_fn(|d| println!("{d:?}"));

    let _ = all!(counter, debug).await;
}
