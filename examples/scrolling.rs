use iocraft::prelude::*;

#[derive(Default, Props)]
struct Props<'a> {
    text: &'a str,
}

#[component]
fn Example<'a>(props: &Props<'a>, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut system = hooks.use_context_mut::<SystemContext>();
    let mut should_exit = hooks.use_state(|| false);
    let mut mouse_captured = hooks.use_state(|| false);

    hooks.use_terminal_events({
        move |event| match event {
            TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
                match code {
                    KeyCode::Char('q') => should_exit.set(true),
                    KeyCode::Char('m') => mouse_captured.set(!mouse_captured.get()),
                    _ => {}
                }
            }
            _ => {}
        }
    });

    if should_exit.get() {
        system.exit();
    }

    system.set_mouse_capture(mouse_captured.get());

    let mouse_status = if mouse_captured.get() { "on" } else { "off" };

    element! {
        View(
            flex_direction: FlexDirection::Column,
            padding: 2,
            align_items: AlignItems::Center
        ) {
            Text(content: format!(
                "Arrow keys/mouse wheel to scroll, 'm' to toggle mouse capture ({}), 'q' to exit",
                mouse_status
            ))
            View(
                border_style: BorderStyle::DoubleLeftRight,
                border_color: Color::Green,
                margin: 1,
                width: 78,
                height: 10,
            ) {
                ScrollView {
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
    smol::block_on(element! { Example(text: text.as_str()) }.render_loop()).unwrap();
}
