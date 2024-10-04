use iocraft::prelude::*;

#[derive(Clone, Copy, Default)]
enum Theme {
    #[default]
    Dark,
    Light,
}

#[derive(Clone, Copy)]
struct ButtonStyle {
    color: Color,
    text_color: Color,
    trim_color: Color,
}

// https://www.ditig.com/publications/256-colors-cheat-sheet
impl Theme {
    fn toggled(&self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    fn background_color(&self) -> Color {
        match self {
            Self::Dark => Color::AnsiValue(237),
            Self::Light => Color::AnsiValue(253),
        }
    }

    fn footer_background_color(&self) -> Color {
        match self {
            Self::Dark => Color::AnsiValue(253),
            Self::Light => Color::AnsiValue(237),
        }
    }

    fn footer_text_color(&self) -> Color {
        match self {
            Self::Dark => Color::AnsiValue(237),
            Self::Light => Color::AnsiValue(253),
        }
    }

    fn screen_color(&self) -> Color {
        Color::AnsiValue(68)
    }

    fn screen_text_color(&self) -> Color {
        Color::AnsiValue(231)
    }

    fn screen_trim_color(&self) -> Color {
        Color::AnsiValue(75)
    }

    fn numpad_button_style(&self) -> ButtonStyle {
        match self {
            Self::Dark => ButtonStyle {
                color: Color::AnsiValue(239),
                text_color: Color::AnsiValue(231),
                trim_color: Color::AnsiValue(243),
            },
            Self::Light => ButtonStyle {
                color: Color::AnsiValue(251),
                text_color: Color::AnsiValue(16),
                trim_color: Color::AnsiValue(255),
            },
        }
    }

    fn operator_button_style(&self) -> ButtonStyle {
        ButtonStyle {
            color: Color::AnsiValue(172),
            text_color: Color::AnsiValue(231),
            trim_color: Color::AnsiValue(215),
        }
    }

    fn clear_button_style(&self) -> ButtonStyle {
        ButtonStyle {
            color: Color::AnsiValue(161),
            text_color: Color::AnsiValue(231),
            trim_color: Color::AnsiValue(205),
        }
    }

    fn fn_button_style(&self) -> ButtonStyle {
        ButtonStyle {
            color: Color::AnsiValue(66),
            text_color: Color::AnsiValue(231),
            trim_color: Color::AnsiValue(115),
        }
    }
}

#[derive(Default, Props)]
struct ScreenProps {
    content: String,
}

#[component]
fn Screen(hooks: Hooks, props: &ScreenProps) -> impl Into<AnyElement<'static>> {
    let theme = hooks.use_context::<Theme>();
    element! {
        Box(
            width: 100pct,
            border_style: BorderStyle::Custom(BorderCharacters {
                top: '▁',
                ..Default::default()
            }),
            border_edges: Edges::Top,
            border_color: theme.screen_trim_color(),
        ) {
            Box(
                width: 100pct,
                background_color: theme.screen_color(),
                padding: 1,
                justify_content: JustifyContent::End,
            ) {
                Text(
                    content: &props.content,
                    align: TextAlign::Right,
                    color: theme.screen_text_color(),
                )
            }
        }
    }
}

#[derive(Default, Props)]
struct CalculatorButtonProps {
    label: String,
    style: Option<ButtonStyle>,
    on_click: Handler<'static, ()>,
}

#[component]
fn CalculatorButton(props: &mut CalculatorButtonProps) -> impl Into<AnyElement<'static>> {
    let style = props.style.unwrap();

    element! {
        Button(handler: props.on_click.take()) {
            Box(
                border_style: BorderStyle::Custom(BorderCharacters {
                    top: '▁',
                    ..Default::default()
                }),
                border_edges: Edges::Top,
                border_color: style.trim_color,
                flex_grow: 1.0,
                margin_left: 1,
                margin_right: 1,
            ) {
                Box(
                    background_color: style.color,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    height: 3,
                    flex_grow: 1.0,
                ) {
                    Text(
                        content: &props.label,
                        color: style.text_color,
                        weight: Weight::Bold,
                    )
                }
            }
        }
    }
}

#[component]
fn Calculator(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let theme = hooks.use_context::<Theme>();
    let numpad_button_style = theme.numpad_button_style();
    let operator_button_style = theme.operator_button_style();
    let fn_button_style = theme.fn_button_style();
    let mut expr = hooks.use_state(|| "0".to_string());
    let mut clear_on_number = hooks.use_state(|| true);

    let mut handle_backspace = move || {
        let new_expr = expr
            .read()
            .chars()
            .take(expr.read().len() - 1)
            .collect::<String>();
        if new_expr.is_empty() {
            expr.set("0".to_string());
            clear_on_number.set(true);
        } else {
            expr.set(new_expr);
            clear_on_number.set(false);
        }
    };

    let mut handle_number = move |n: u8| {
        if clear_on_number.get() {
            expr.set(n.to_string());
            clear_on_number.set(false);
        } else {
            expr.set(expr.to_string() + &n.to_string());
        }
    };

    let mut handle_decimal = move || {
        if clear_on_number.get() {
            expr.set("0.".to_string());
            clear_on_number.set(false);
        } else if expr.read().chars().last() != Some('.') {
            expr.set(expr.to_string() + ".");
        }
    };

    let mut handle_clear = move || {
        expr.set("0".to_string());
        clear_on_number.set(true);
    };

    let has_trailing_operator = matches!(
        expr.read().chars().last(),
        Some('+') | Some('-') | Some('×') | Some('÷')
    );

    let mut handle_operator = move |op: char| {
        if clear_on_number.get() {
            clear_on_number.set(false);
        }
        if !has_trailing_operator {
            expr.set(expr.to_string() + &op.to_string());
        }
    };

    let mut handle_percent = move || {
        if clear_on_number.get() {
            clear_on_number.set(false);
        }
        if !has_trailing_operator {
            expr.set(expr.to_string() + "%");
        }
    };

    let mut handle_plus_minus = move || {
        if clear_on_number.get() {
            clear_on_number.set(false);
        }
        if !has_trailing_operator {
            expr.set(format!("-({})", expr));
        }
    };

    let mut handle_equals = move || {
        if let Ok(f) = mexprp::eval::<f64>(&expr.to_string()) {
            expr.set(f.to_string());
            clear_on_number.set(true);
        }
    };

    hooks.use_terminal_events({
        move |event| match event {
            TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
                match code {
                    KeyCode::Char('/') => handle_operator('÷'),
                    KeyCode::Char('*') => handle_operator('×'),
                    KeyCode::Char('+') => handle_operator('+'),
                    KeyCode::Char('-') => handle_operator('-'),
                    KeyCode::Char('0') => handle_number(0),
                    KeyCode::Char('1') => handle_number(1),
                    KeyCode::Char('2') => handle_number(2),
                    KeyCode::Char('3') => handle_number(3),
                    KeyCode::Char('4') => handle_number(4),
                    KeyCode::Char('5') => handle_number(5),
                    KeyCode::Char('6') => handle_number(6),
                    KeyCode::Char('7') => handle_number(7),
                    KeyCode::Char('8') => handle_number(8),
                    KeyCode::Char('9') => handle_number(9),
                    KeyCode::Char('.') => handle_decimal(),
                    KeyCode::Char('%') => handle_percent(),
                    KeyCode::Char('=') | KeyCode::Enter => handle_equals(),
                    KeyCode::Backspace => handle_backspace(),
                    KeyCode::Char('c') => handle_clear(),
                    _ => {}
                }
            }
            _ => {}
        }
    });

    element! {
        Box(
            width: 100pct,
            height: 100pct,
            flex_direction: FlexDirection::Column,
            padding_left: 1,
            padding_right: 1,
        ) {
            Box(
                padding_left: 1,
                padding_right: 1,
            ) {
                Screen(content: expr.to_string())
            }
            Box(width: 100pct) {
                CalculatorButton(label: "←", style: fn_button_style, on_click: move |_| handle_backspace())
                CalculatorButton(label: "±", style: fn_button_style, on_click: move |_| handle_plus_minus())
                CalculatorButton(label: "%", style: fn_button_style, on_click: move |_| handle_percent())
                CalculatorButton(label: "÷", style: operator_button_style, on_click: move |_| handle_operator('÷'))
            }
            Box(width: 100pct) {
                CalculatorButton(label: "7", style: numpad_button_style, on_click: move |_| handle_number(7))
                CalculatorButton(label: "8", style: numpad_button_style, on_click: move |_| handle_number(8))
                CalculatorButton(label: "9", style: numpad_button_style, on_click: move |_| handle_number(9))
                CalculatorButton(label: "×", style: operator_button_style, on_click: move |_| handle_operator('×'))
            }
            Box(width: 100pct) {
                CalculatorButton(label: "4", style: numpad_button_style, on_click: move |_| handle_number(4))
                CalculatorButton(label: "5", style: numpad_button_style, on_click: move |_| handle_number(5))
                CalculatorButton(label: "6", style: numpad_button_style, on_click: move |_| handle_number(6))
                CalculatorButton(label: "-", style: operator_button_style, on_click: move |_| handle_operator('-'))
            }
            Box(width: 100pct) {
                CalculatorButton(label: "1", style: numpad_button_style, on_click: move |_| handle_number(1))
                CalculatorButton(label: "2", style: numpad_button_style, on_click: move |_| handle_number(2))
                CalculatorButton(label: "3", style: numpad_button_style, on_click: move |_| handle_number(3))
                CalculatorButton(label: "+", style: operator_button_style, on_click: move |_| handle_operator('+'))
            }
            Box(width: 100pct) {
                CalculatorButton(label: "C", style: theme.clear_button_style(), on_click: move |_| handle_clear())
                CalculatorButton(label: "0", style: numpad_button_style, on_click: move |_| handle_number(0))
                CalculatorButton(label: ".", style: numpad_button_style, on_click: move |_| handle_decimal())
                CalculatorButton(label: "=", style: operator_button_style, on_click: move |_| handle_equals())
            }
        }
    }
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let (width, height) = hooks.use_terminal_size();
    let mut system = hooks.use_context_mut::<SystemContext>();
    let mut should_exit = hooks.use_state(|| false);
    let mut theme = hooks.use_state(|| Theme::default());

    hooks.use_terminal_events({
        move |event| match event {
            TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
                match code {
                    KeyCode::Char('q') => should_exit.set(true),
                    KeyCode::Char('t') => theme.set(theme.get().toggled()),
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
        Box(
            // subtract one in case there's a scrollbar
            width: width - 1,
            height: height,
            background_color: theme.get().background_color(),
            flex_direction: FlexDirection::Column,
            gap: 1,
        ) {
            Box(
                flex_grow: 1.0,
            ) {
                Box(
                    max_width: 120,
                    max_height: 40,
                    flex_grow: 1.0,
                ) {
                    ContextProvider(value: Context::owned(theme.get())) {
                        Calculator
                    }
                }
            }
            Box(
                height: 1,
                background_color: theme.get().footer_background_color(),
                padding_left: 1,
            ) {
                Text(content: "[T] Toggle Theme [Q] Quit", color: theme.get().footer_text_color())
            }
        }
    }
}

fn main() {
    smol::block_on(element!(App).fullscreen()).unwrap();
}
