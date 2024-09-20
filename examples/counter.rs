use iocraft::prelude::*;
use std::time::Duration;

#[state]
struct CounterState {
    count: Signal<i32>,
}

#[component]
fn Counter(mut state: CounterState, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    hooks.use_future(async move {
        loop {
            smol::Timer::after(Duration::from_millis(100)).await;
            state.count += 1;
        }
    });

    element! {
        Text(color: Color::Blue, content: format!("counter: {}", state.count))
    }
}

fn main() {
    smol::block_on(element!(Counter).render_loop()).unwrap();
}
