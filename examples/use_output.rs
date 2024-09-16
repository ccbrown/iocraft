use iocraft::prelude::*;
use std::time::Duration;

#[hooks]
struct ExampleHooks {
    run_loop: UseAsync,
    output: UseOutput,
}

#[component]
fn Example(hooks: &mut ExampleHooks) -> impl Into<AnyElement> {
    let stdout = hooks.output.use_stdout();
    let stderr = hooks.output.use_stderr();

    hooks.run_loop.spawn_once(|| async move {
        loop {
            smol::Timer::after(Duration::from_secs(1)).await;
            stdout.println("Hello from iocraft to stdout!");
            stderr.println("  And hello to stderr too!");
        }
    });

    element! {
        Box(border_style: BorderStyle::Round, border_color: Color::Green) {
            Text(content: "Hello, use_stdio!")
        }
    }
}

fn main() {
    smol::block_on(element!(Example).render_loop()).unwrap();
}
