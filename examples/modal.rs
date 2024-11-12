use iocraft::prelude::*;

#[component]
fn Popup(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let (width, height) = hooks.use_terminal_size();
    let mut system = hooks.use_context_mut::<SystemContext>();
    let mut should_popup = hooks.use_state(|| false);
    let mut should_exit = hooks.use_state(|| false);

    hooks.use_terminal_events(move |event| match event {
        TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
            match code {
                KeyCode::Char('x') => should_popup.set(false),
                KeyCode::Char('q') => should_exit.set(true),
                KeyCode::Char('s') => should_popup.set(true),
                _ => {}
            }
        }
        _ => {}
    });

    if should_exit.get() {
        system.exit();
    }

    element! {
        Box(
            width: width - 1,
            height: height,
            flex_direction: FlexDirection::Column,
            border_style: BorderStyle::Round,
            border_color: Color::Magenta,
            gap: 1
        ) {
            Box(
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                margin: 2,
            ) {
                Box(
                    align_items: AlignItems::Center,
                ) {
                    Text(content: "Press 's' to display a popup and 'x' to exit!")
                }
                Box(
                    align_items: AlignItems::Center,
                ) {
                    Text(content: "Press 'q' to exit!")
                }
                Box(
                    width: 78
                ) {
                    Text(
                        align: TextAlign::Center,
                        wrap: TextWrap::Wrap,
                        content: "Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry's standard dummy text ever since the 1500s, when an unknown printer took a galley of type and scrambled it to make a type specimen book. It has survived not only five centuries, but also the leap into electronic typesetting, remaining essentially unchanged. It was popularised in the 1960s with the release of Letraset sheets containing Lorem Ipsum passages, and more recently with desktop publishing software like Aldus PageMaker including versions of Lorem Ipsum."
                    )
                }
                Modal(is_open: should_popup.get()) {
                    Box(
                        align_items: AlignItems::Center,
                        border_style: BorderStyle::Round,
                        border_color: Color::Green,
                        position: Position::Absolute
                    ) {
                        Text(content: "안영하세요!")
                    }
                }
            }
        }
    }
}

fn main() {
    smol::block_on(element!(Popup).fullscreen()).unwrap();
}
