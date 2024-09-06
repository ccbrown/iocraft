use iocraft::prelude::*;
use std::time::Duration;

#[state]
struct CounterState {
    count: Signal<i32>,
}

#[hooks]
struct CounterHooks {
    run_loop: UseFuture,
}

#[component]
fn Counter(state: &CounterState, hooks: &mut CounterHooks) -> impl Into<AnyElement> {
    hooks.run_loop.use_future({
        let mut count = state.count.clone();
        || async move {
            loop {
                smol::Timer::after(Duration::from_millis(100)).await;
                count += 1;
            }
        }
    });

    element! {
        Text(color: Color::DarkBlue, content: format!("counter: {}", state.count))
    }
}

fn main() {
    smol::block_on(element!(Counter).render());
}
