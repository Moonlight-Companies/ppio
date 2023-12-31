use async_trait::async_trait;
use ppio::prelude::*;

struct Greeting;

#[async_trait]
impl Poll for Greeting {
    type Item = &'static str;

    async fn poll(&mut self, tx: Sender<Self::Item>) -> PollOutput {
        use tokio::time::{sleep, Duration};

        loop {
            tx.send("Hello!").await?;
            sleep(Duration::from_secs(5)).await;

            tx.send("Sup!").await?;
            sleep(Duration::from_secs(5)).await;

            tx.send("Hi!").await?;
            sleep(Duration::from_secs(5)).await;
        }
    }
}

#[derive(Default)]
struct Greeter(Option<&'static str>);

impl State for Greeter {
    type State = &'static str;

    fn update(&mut self, state: Self::State) {
        self.0 = Some(state);
    }
}

#[async_trait]
impl Poll for Greeter {
    type Item = &'static str;

    async fn poll(&mut self, tx: Sender<Self::Item>) -> PollOutput {
        use tokio::time::{sleep, Duration};

        if let Some(greeting) = self.0 {
            loop {
                tx.send(greeting).await?;
                sleep(Duration::from_secs(1)).await;
            }
        }

        futures::future::pending().await
    }
}

#[tokio::main]
async fn main() {
    let (greeting, rx) = poll(Greeting);
    let (greeter, rx) = poll(Greeter::default()).with_state(rx);
    let printer = push(rx).to_fn(|d| println!("{d:?}"));

    let _ = allt!(greeting, greeter, printer).await;
}
