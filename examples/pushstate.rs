use ppio::prelude::*;
use ppio::channel;
use tokio::time::{Duration, interval};

#[derive(Default)]
pub struct Counter;

impl Poll for Counter {
    type Item = usize;

    async fn poll(&mut self, tx: channel::Sender<Self::Item>) -> PollOutput {
        let mut count = 0;
        let mut timer = interval(Duration::from_millis(500));

        loop {
            timer.tick().await;
            tx.send(count).await?;
            count += 1;
        }
    }
}

pub struct Filter;

#[derive(Clone, Copy, Debug, Default)]
pub enum Parity {
    Even,
    #[default]
    Odd
}

impl Parity {
    pub fn other(&self) -> Self {
        match self {
            Self::Even => Self::Odd,
            Self::Odd => Self::Even
        }
    }

    pub fn is_even(&self) -> bool {
        match self {
            Self::Even => true,
            Self::Odd => false
        }
    }
}

impl Poll for Filter {
    type Item = Parity;

    async fn poll(&mut self, tx: channel::Sender<Self::Item>) -> PollOutput {
        let mut parity = Parity::Even;
        let mut timer = interval(Duration::from_millis(2500));

        loop {
            timer.tick().await;
            tx.send(parity).await?;
            parity = parity.other();
        }
    }
}

#[derive(Default)]
pub struct Announcer(Parity);

impl State<Parity> for Announcer {
    fn update(&mut self, state: Parity) {
        self.0 = state;
    }
}

impl Push<usize> for Announcer {
    async fn push(&mut self, item: usize) -> PushOutput {
        if self.0.is_even() && item % 2 == 0 {
            println!("even: {}", item);
        } else if !self.0.is_even() && item % 2 != 0 {
            println!("odd : {}", item);
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let (counter, rx) = poll(Counter);
    let (filter, parity) = poll(Filter);
    let debug = push(rx).to(Announcer::default()).with_state(parity);

    let _ = all!(counter, filter, debug).await;
}
