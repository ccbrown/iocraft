use iocraft::prelude::*;
use std::time::Duration;

#[component]
fn ProgressBar(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut system = hooks.use_context_mut::<SystemContext>();
    let mut progress = hooks.use_state::<f32, _>(|| 0.0);

    hooks.use_future(async move {
        loop {
            smol::Timer::after(Duration::from_millis(100)).await;
            progress.set((progress.get() + 2.0).min(100.0));
        }
    });

    if progress >= 100.0 {
        system.exit();
    }

    element! {
        Box {
            Box(border_style: BorderStyle::Round, border_color: Color::Blue, width: 60) {
                Box(width: Percent(progress.get()), height: 1, background_color: Color::Green)
            }
            Box(padding: 1) {
                Text(content: format!("{:.0}%", progress))
            }
        }
    }
}

fn main() {
    smol::block_on(element!(ProgressBar).render_loop()).unwrap();
    println!("done!");
}
