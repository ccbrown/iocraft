use flashy_cli::prelude::*;
use std::time::Duration;

struct CounterState {
    count: Value<i32>,
}

struct Counter {
    state: CounterState,
}

impl Component for Counter {
    type State = CounterState;

    fn new() -> Self {
        Self {
            state: Self::State {
                count: Value::new_with_default(Signal),
            },
        }
    }

    fn render(&self) {
        println!("counter: {}", self.state.count)
    }

    async fn wait(&mut self) {
        smol::Timer::after(Duration::from_secs(1)).await;
        self.state.count.set(*self.state.count + 1);
    }
}

fn main() {
    smol::block_on(render::<Counter>());
}
