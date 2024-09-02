use flashy_cli::prelude::*;
use std::time::Duration;

struct CounterState {
    count: i32,
}

struct Counter {
    children: Components<Text>,
    state: CounterState,
}

impl Component for Counter {
    type Props = ();
    type State = CounterState;

    fn new(_props: Self::Props) -> Self {
        Self {
            children: Components::default(),
            state: Self::State { count: 0 },
        }
    }

    fn update(&mut self, _props: Self::Props) {}

    fn render(&mut self) {
        let mut renderer = self.children.renderer();
        renderer.render(TextProps {
            value: format!("counter: {}", self.state.count),
        });
    }

    async fn wait(&mut self) {
        smol::Timer::after(Duration::from_secs(1)).await;
        self.state.count = self.state.count + 1;
    }
}

fn main() {
    smol::block_on(render::<Counter>(()));
}
