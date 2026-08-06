//! Minimal reproduction of a row-duplication bug in iocraft's inline renderer.
//!
//! Run with:  cargo run -p iocraft-examples --example duplicate  (in a real terminal)
//!
//! A `View` with `padding: 1, row_gap: 1` holds a growing list of text lines above a
//! single static line. Pressing Enter appends one line. Growing the list this way
//! duplicates the static line (and prior lines) on screen / in scrollback.
//! Also new lines are added without a row gap.
//!
//! Removing the static line, or either of `padding`/`row_gap`, makes it clean.

use iocraft::prelude::*;

#[component]
fn Duplicate(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut lines = hooks.use_state(|| vec!["first line".to_string()]);

    hooks.use_terminal_events(move |event| {
        if let TerminalEvent::Key(KeyEvent {
            code: KeyCode::Enter,
            kind: KeyEventKind::Press,
            ..
        }) = event
        {
            lines.write().push("new line".to_string());
        }
    });

    element! {
        View(flex_direction: FlexDirection::Column, padding: 1, row_gap: 1) {
            #(lines.read().iter().map(|line| {
                element! { Text(content: line.as_str()) }
            }))
            Text(content: "static line below")
        }
    }
}

fn main() {
    smol::block_on(element!(Duplicate).render_loop()).unwrap();
}
