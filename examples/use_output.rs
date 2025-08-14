use iocraft::prelude::*;
use std::time::Duration;

#[component]
fn Example(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let (stdout, stderr) = hooks.use_output();

    hooks.use_future(async move {
        loop {
            smol::Timer::after(Duration::from_secs(1)).await;
            stdout.println("Hello from iocraft to stdout!");
            stderr.println("  And hello to stderr too!");
            stdout.print("Using print: ");
            stdout.print("part1 ");
            stdout.print("part2 ");
            stdout.println("done!");
        }
    });

    element! {
        View(border_style: BorderStyle::Round, border_color: Color::Green) {
            Text(content: "Hello, use_stdio!")
        }
    }
}

fn main() {
    smol::block_on(element!(Example).render_loop()).unwrap();
}
