use iocraft::prelude::*;

#[props]
struct FormFieldProps {
    label: String,
    value: Option<Signal<String>>,
    has_focus: bool,
}

#[component]
fn FormField(props: &FormFieldProps) -> impl Into<AnyElement<'static>> {
    let Some(value) = props.value else {
        panic!("value is required");
    };

    element! {
        Box(
            border_style: if props.has_focus { BorderStyle::Round } else { BorderStyle::None },
            border_color: Color::Blue,
            padding_left: if props.has_focus { 0 } else { 1 },
            padding_right: if props.has_focus { 0 } else { 1 },
        ) {
            Box(width: 15) {
                Text(content: format!("{}: ", props.label))
            }
            Box(
                background_color: Color::DarkGrey,
                width: 30,
            ) {
                TextInput(
                    has_focus: props.has_focus,
                    value: value.to_string(),
                    on_change: move |new_value| value.set(new_value),
                )
            }
        }
    }
}

#[context]
struct FormContext<'a> {
    system: &'a mut SystemContext,
}

#[state]
struct FormState {
    first_name: Signal<String>,
    last_name: Signal<String>,
    focus: Signal<i32>,
    should_submit: Signal<bool>,
}

#[props]
struct FormProps<'a> {
    first_name_out: Option<&'a mut String>,
    last_name_out: Option<&'a mut String>,
}

#[component]
fn Form<'a>(
    props: &mut FormProps<'a>,
    state: FormState,
    mut hooks: Hooks,
    context: FormContext,
) -> impl Into<AnyElement<'static>> {
    hooks.use_terminal_events(move |event| match event {
        TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
            match code {
                KeyCode::Enter => state.should_submit.set(true),
                KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                    state.focus.set((state.focus + 1) % 2)
                }
                _ => {}
            }
        }
        _ => {}
    });

    if state.should_submit.get() {
        if let Some(first_name_out) = props.first_name_out.as_mut() {
            **first_name_out = state.first_name.to_string();
        }
        if let Some(last_name_out) = props.last_name_out.as_mut() {
            **last_name_out = state.last_name.to_string();
        }
        context.system.exit();
        element!(Box)
    } else {
        element! {
            Box(
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                margin: 2,
            ) {
                Box(
                    padding_bottom: if state.focus == 0 { 1 } else { 2 },
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                ) {
                    Text(content: "What's your name?", color: Color::White, weight: Weight::Bold)
                    Text(content: "Press tab to cycle through fields.\nPress enter to submit.", color: Color::Grey, align: TextAlign::Center)
                }
                FormField(label: "First Name", value: state.first_name, has_focus: state.focus == 0)
                FormField(label: "Last Name", value: state.last_name, has_focus: state.focus == 1)
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
        println!("Hello, {} {}!", first_name, last_name);
    }
}
