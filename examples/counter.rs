use flashy_io::prelude::*;
use futures::future::{select, BoxFuture, FutureExt};
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
    children: Components,
    state: CounterState,
}

impl ElementType for Counter {
    type Props = CounterProps;
}

impl Component for Counter {
    type Props = CounterProps;
    type State = CounterState;

    fn new(_props: Self::Props) -> Self {
        Self {
            children: Components::default(),
            state: Self::State { count: 0 },
        }
    }

    fn set_props(&mut self, _props: Self::Props) {}

    fn update(&mut self, updater: ComponentUpdater<'_>) {
        let mut updater = self.children.updater(updater);
        updater.update(flashy! {
            Text(color: Color::DarkBlue, content: format!("counter: {}", self.state.count))
        });
    }

    fn render(&self, renderer: ComponentRenderer<'_>) {
        self.children.render(renderer)
    }

    fn wait(&mut self) -> BoxFuture<()> {
        async {
            select(
                smol::Timer::after(Duration::from_millis(100)),
                self.children.wait().boxed(),
            )
            .await;
            self.state.count = self.state.count + 1;
        }
        .boxed()
    }
}

fn main() {
    smol::block_on(flashy!(Counter).render());
}
