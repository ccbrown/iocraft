use iocraft::prelude::*;
use std::time::Duration;

#[state]
struct ProgressBarState {
    progress: Signal<f32>,
}

#[hooks]
struct ProgressBarHooks {
    run_loop: UseFuture,
}

#[component]
fn ProgressBar(
    state: &ProgressBarState,
    hooks: &mut ProgressBarHooks,
) -> impl Into<AnyElement<'static>> {
    hooks.run_loop.use_future({
        let progress = state.progress.clone();
        || async move {
            loop {
                smol::Timer::after(Duration::from_millis(100)).await;
                progress.set((progress.get() + 2.0).min(100.0));
            }
        }
    });

    element! {
        Box {
            Box(border_style: BorderStyle::Round, border_color: Color::Blue, width: 60) {
                Box(width: Percent(state.progress.get()), height: 1, background_color: Color::Green)
            }
            Box(padding: 1) {
                Text(content: format!("{:.0}%", state.progress))
            }
        }
    }
}

fn main() {
    smol::block_on(element!(ProgressBar).render_loop()).unwrap();
}
