use iocraft::prelude::*;

#[derive(Default, Props)]
struct Props<'a> {
    text: &'a str,
}

#[component]
fn Example<'a>(props: &Props<'a>, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut system = hooks.use_context_mut::<SystemContext>();
    let mut scroll_offset = hooks.use_state(|| 0i32);
    let mut should_exit = hooks.use_state(|| false);

    hooks.use_terminal_events({
        move |event| match event {
            TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
                match code {
                    KeyCode::Char('q') => should_exit.set(true),
                    KeyCode::Up => scroll_offset.set((scroll_offset.get() - 1).max(0)),
                    KeyCode::Down => scroll_offset.set(scroll_offset.get() + 1),
                    _ => {}
                }
            }
            _ => {}
        }
    });

    if should_exit.get() {
        system.exit();
    }

    element! {
        View(
            flex_direction: FlexDirection::Column,
            padding: 2,
            align_items: AlignItems::Center
        ) {
            Text(content: "Use arrow keys to scroll. Press \"q\" to exit.")
            View(
                border_style: BorderStyle::DoubleLeftRight,
                border_color: Color::Green,
                margin: 1,
                width: 78,
                height: 10,
                overflow: Overflow::Hidden,
            ) {
                View(
                    position: Position::Absolute,
                    top: -scroll_offset.get(),
                ) {
                    Text(content: props.text)
                }
            }
        }
    }
}

fn main() {
    let mut text = String::new();
    for i in 0..100 {
        text.push_str(&format!("Line {}\n", i));
    }
    smol::block_on(
        element! {
            Example(text: text.as_str())
        }
        .render_loop(),
    )
    .unwrap();
}
