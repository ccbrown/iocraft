use flashy_io::prelude::*;
use std::time::Duration;

#[derive(Clone, Default)]
struct CounterProps {}

struct Counter {
    count: i32,
}

impl Component for Counter {
    type Props = CounterProps;

    fn new(_props: Self::Props) -> Self {
        Self { count: 0 }
    }

    fn set_props(&mut self, _props: Self::Props) {}

    fn update(&self, updater: &mut ComponentUpdater<'_>) {
        updater.update_children([flashy! {
            Text(color: Color::DarkBlue, content: format!("counter: {}", self.count))
        }]);
    }

    async fn wait(&mut self) {
        smol::Timer::after(Duration::from_millis(100)).await;
        self.count = self.count + 1;
    }
}

fn main() {
    smol::block_on(flashy!(Counter).render());
}
