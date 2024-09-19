use iocraft::prelude::*;
use std::time::Duration;

#[context]
struct ProgressBarContext<'a> {
    system: &'a mut SystemContext,
}

#[state]
struct ProgressBarState {
    progress: Signal<f32>,
}

#[hooks]
struct ProgressBarHooks {
    run_loop: UseAsync,
}

#[component]
fn ProgressBar(
    state: ProgressBarState,
    hooks: &mut ProgressBarHooks,
    context: ProgressBarContext,
) -> impl Into<AnyElement<'static>> {
    hooks.run_loop.spawn_once(move || async move {
        loop {
            smol::Timer::after(Duration::from_millis(100)).await;
            state.progress.set((state.progress.get() + 2.0).min(100.0));
        }
    });

    if state.progress >= 100.0 {
        context.system.exit();
    }

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
    println!("done!");
}
