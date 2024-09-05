use flashy_io::prelude::*;
use futures::future::{BoxFuture, FutureExt};
use std::time::Duration;

#[derive(Clone, Default)]
struct CounterProps {}

impl ComponentProps for CounterProps {
    type Component = Counter;
}

struct CounterState {
    count: i32,
}

struct Counter {
    state: CounterState,
}

impl Component for Counter {
    type Props = CounterProps;
    type State = CounterState;

    fn new(_props: Self::Props) -> Self {
        Self {
            state: Self::State { count: 0 },
        }
    }

    fn set_props(&mut self, _props: Self::Props) {}

    fn update(&self, updater: &mut ComponentUpdater<'_>) {
        updater.update_children([flashy! {
            Text(color: Color::DarkBlue, content: format!("counter: {}", self.state.count))
        }]);
    }

    fn wait(&mut self) -> BoxFuture<()> {
        async {
            smol::Timer::after(Duration::from_millis(100)).await;
            self.state.count = self.state.count + 1;
        }
        .boxed()
    }
}

fn main() {
    smol::block_on(flashy!(Counter).render());
}
