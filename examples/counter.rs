use flashy_cli::prelude::*;
use futures::future::{select, BoxFuture, FutureExt};
use std::time::Duration;

struct CounterState {
    count: i32,
}

struct Counter {
    node_id: NodeId,
    children: Components<Text>,
    state: CounterState,
}

impl Renderable for Counter {
    fn update(&mut self, updater: TreeUpdater<'_>) {
        let mut updater = self.children.updater(updater);
        updater.update(TextProps {
            value: format!("counter: {}", self.state.count),
        });
    }

    fn render(&self, renderer: TreeRenderer<'_>) {
        self.children.render(renderer)
    }

    fn wait(&mut self) -> BoxFuture<()> {
        async {
            select(
                smol::Timer::after(Duration::from_millis(200)),
                self.children.wait().boxed(),
            )
            .await;
            self.state.count = self.state.count + 1;
        }
        .boxed()
    }
}

impl Component for Counter {
    type Props = ();
    type State = CounterState;

    fn new(node_id: NodeId, _props: Self::Props) -> Self {
        Self {
            node_id,
            children: Components::default(),
            state: Self::State { count: 0 },
        }
    }

    fn set_props(&mut self, _props: Self::Props) {}

    fn node_id(&self) -> NodeId {
        self.node_id
    }
}

fn main() {
    smol::block_on(render::<Counter>(()));
}
