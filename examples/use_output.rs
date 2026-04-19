use iocraft::prelude::*;
use std::time::Duration;

#[component]
fn Example(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let (stdout, stderr) = hooks.use_output();

    hooks.use_future(async move {
        stdout.println("Hello from iocraft to stdout!");
        stderr.println("  And hello to stderr too!");

        stdout.print("Working...");
        for _ in 0..5 {
            smol::Timer::after(Duration::from_secs(1)).await;
            stdout.print(".");
        }
        stdout.println("\nDone!");
    });

    element! {
        View(border_style: BorderStyle::Round, border_color: Color::Green) {
            Text(content: "Hello, use_output!")
        }
    }
}

fn main() {
    smol::block_on(element!(Example).render_loop()).unwrap();
}
