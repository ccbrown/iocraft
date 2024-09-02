use flashy_cli::prelude::*;
use std::time::Duration;

struct CounterState {
    count: i32,
}

struct Counter {
    state: CounterState,
}

impl Component for Counter {
    type Props = ();
    type State = CounterState;

    fn new(_props: Self::Props) -> Self {
        Self {
            state: Self::State { count: 0 },
        }
    }

    fn render(&self) {
        println!("counter: {}", self.state.count)
    }

    async fn wait(&mut self) {
        smol::Timer::after(Duration::from_secs(1)).await;
        self.state.count = self.state.count + 1;
    }
}

fn main() {
    smol::block_on(render::<Counter>(()));
}
