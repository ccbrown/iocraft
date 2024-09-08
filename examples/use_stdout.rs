use iocraft::prelude::*;
use std::time::Duration;

#[hooks]
struct ExampleHooks {
    run_loop: UseFuture,
    output: UseStdout,
}

#[component]
fn Example(hooks: &mut ExampleHooks) -> impl Into<AnyElement> {
    let output = hooks.output.use_stdout();

    hooks.run_loop.use_future(|| async move {
        loop {
            smol::Timer::after(Duration::from_secs(1)).await;
            output.println("Hello from iocraft to stdout!");
        }
    });

    element! {
        Box(border_style: BorderStyle::Round, border_color: Color::Green) {
            Text(content: "Hello, use_stdout!")
        }
    }
}

fn main() {
    smol::block_on(element!(Example).render_loop()).unwrap();
}
