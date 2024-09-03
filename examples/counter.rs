use flashy_cli::prelude::*;
use futures::future::{select, BoxFuture, FutureExt};
use std::time::Duration;

struct CounterProps {}

impl ComponentProps for CounterProps {
    type Component = Counter;
}

struct CounterState {
    count: i32,
}

struct Counter {
    node_id: NodeId,
    children: Components,
    state: CounterState,
}

impl Component for Counter {
    type Props = CounterProps;
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

    fn update(&mut self, updater: ComponentUpdater<'_>) {
        let mut updater = self.children.updater(updater);
        updater.update(ComponentDeclaration::new(
            "text",
            TextProps {
                value: format!("counter: {}", self.state.count),
            },
        ));
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
    smol::block_on(render_dynamic(CounterProps {}));
}
