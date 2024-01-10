use ppio::prelude::*;

struct Greeting;

impl Poll for Greeting {
    type Item = &'static str;

    async fn poll(&mut self, tx: ppio::channel::Sender<Self::Item>) -> PollOutput {
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

impl Poll for Greeter {
    type Item = &'static str;

    async fn poll(&mut self, tx: ppio::channel::Sender<Self::Item>) -> PollOutput {
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
    let (greeting, [str, announcer]) = poll(Greeting).broadcast();
    let announcer = push(announcer).to_fn(|str| println!("got new str: {str}"));

    let (greeter, rx) = poll(Greeter::default()).with_state(str);
    let printer = push(rx).to_fn(|d| println!("{d}"));

    let _ = all!(greeting, announcer, greeter, printer).await;
}
