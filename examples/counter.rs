use iocraft::prelude::*;
use std::time::Duration;

#[state]
struct CounterState {
    count: Signal<i32>,
}

#[hooks]
struct CounterHooks {
    run_loop: UseAsync,
}

#[component]
fn Counter(mut state: CounterState, hooks: &mut CounterHooks) -> impl Into<AnyElement<'static>> {
    hooks.run_loop.spawn_once(move || async move {
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
