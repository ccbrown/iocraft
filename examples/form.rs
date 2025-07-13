use iocraft::prelude::*;

#[derive(Default, Props)]
struct FormFieldProps {
    label: String,
    value: Option<State<String>>,
    has_focus: bool,
    multiline: bool,
}

#[component]
fn FormField(props: &FormFieldProps) -> impl Into<AnyElement<'static>> {
    let Some(mut value) = props.value else {
        panic!("value is required");
    };

    element! {
        View(
            border_style: if props.has_focus { BorderStyle::Round } else { BorderStyle::None },
            border_color: Color::Blue,
            padding: if props.has_focus { 0 } else { 1 },
        ) {
            View(width: 15) {
                Text(content: format!("{}: ", props.label))
            }
            View(
                background_color: Color::DarkGrey,
                width: 30,
                height: if props.multiline { 5 } else { 1 },
            ) {
                TextInput(
                    has_focus: props.has_focus,
                    value: value.to_string(),
                    on_change: move |new_value| value.set(new_value),
                    multiline: props.multiline,
                )
            }
        }
    }
}

#[derive(Default, Props)]
struct FormProps<'a> {
    first_name_out: Option<&'a mut String>,
    last_name_out: Option<&'a mut String>,
}

#[component]
fn Form<'a>(props: &mut FormProps<'a>, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut system = hooks.use_context_mut::<SystemContext>();

    let first_name = hooks.use_state(|| "".to_string());
    let last_name = hooks.use_state(|| "".to_string());
    let life_story = hooks.use_state(|| "".to_string());
    let mut focus = hooks.use_state(|| 0);
    let mut should_submit = hooks.use_state(|| false);

    hooks.use_terminal_events(move |event| match event {
        TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
            match code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if focus == 3 {
                        should_submit.set(true)
                    }
                }
                KeyCode::BackTab => focus.set((focus + 3) % 4),
                KeyCode::Tab => focus.set((focus + 1) % 4),
                KeyCode::Up if focus != 0 => focus.set((focus + 3) % 4),
                KeyCode::Down if focus != 2 => focus.set((focus + 1) % 4),
                _ => {}
            }
        }
        _ => {}
    });

    if should_submit.get() {
        if let Some(first_name_out) = props.first_name_out.as_mut() {
            **first_name_out = first_name.to_string();
        }
        if let Some(last_name_out) = props.last_name_out.as_mut() {
            **last_name_out = last_name.to_string();
        }
        system.exit();
        element!(View)
    } else {
        element! {
            View(
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                margin: 2,
            ) {
                View(
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    margin_bottom: 1,
                ) {
                    Text(content: "What's your name?", color: Color::White, weight: Weight::Bold)
                    Text(content: "Press tab to cycle through fields.", color: Color::Grey, align: TextAlign::Center)
                }
                FormField(label: "First Name", value: first_name, has_focus: focus == 0)
                FormField(label: "Last Name", value: last_name, has_focus: focus == 1)
                FormField(label: "Life Story", value: life_story, has_focus: focus == 2, multiline: true)
                View(
                    border_style: if focus == 3 { BorderStyle::Round } else { BorderStyle::None },
                    border_color: Color::Green,
                    padding: if focus == 3 { 0 } else { 1 },
                ) {
                    Text(content: "Submit", color: Color::White, weight: Weight::Bold)
                }
            }
        }
    }
}

fn main() {
    let mut first_name = String::new();
    let mut last_name = String::new();
    smol::block_on(
        element! {
            Form(
                first_name_out: &mut first_name,
                last_name_out: &mut last_name,
            )
        }
        .render_loop(),
    )
    .unwrap();
    if first_name.is_empty() && last_name.is_empty() {
        println!("No name entered.");
    } else {
        println!(
            "Hello, {} {}! What a fascinating life story!",
            first_name, last_name
        );
    }
}
