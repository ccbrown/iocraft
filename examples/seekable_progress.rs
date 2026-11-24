use iocraft::{hooks::UseComponentRect, prelude::*};

#[derive(Default, Props)]
struct SeekBarProps {
    percent: u16,
    on_seek: HandlerMut<'static, u16>,
}

#[component]
fn SeekBar(mut hooks: Hooks, props: &mut SeekBarProps) -> impl Into<AnyElement<'static>> {
    let width = hooks.use_component_rect().map_or(0, |r| r.right - r.left);
    let mut on_seek = props.on_seek.take();
    hooks.use_local_terminal_events(move |event| {
        if let TerminalEvent::FullscreenMouse(FullscreenMouseEvent {
            column,
            kind: MouseEventKind::Down(MouseButton::Left),
            ..
        }) = event
        {
            // The hook gives us a column relative to the bar, not the terminal.
            let last_column = width - 1;
            if last_column > 0 {
                on_seek((i32::from(column) * 100 / last_column) as u16);
            }
        }
    });

    element! {
        View(width: Percent(100.0), height: 1, background_color: Color::DarkGrey) {
            View(width: Percent(f32::from(props.percent)), height: 1, background_color: Color::Cyan)
        }
    }
}

#[component]
fn Example(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut percent = hooks.use_state(|| 25u16);
    let mut quit = hooks.use_state(|| false);
    let mut system = hooks.use_context_mut::<SystemContext>();
    let (width, _) = hooks.use_terminal_size();

    hooks.use_terminal_events(move |event| {
        if let TerminalEvent::Key(KeyEvent { code, kind, .. }) = event {
            if kind != KeyEventKind::Release {
                match code {
                    KeyCode::Left => percent.set(percent.get().saturating_sub(5)),
                    KeyCode::Right => percent.set((percent.get() + 5).min(100)),
                    KeyCode::Char('q') => quit.set(true),
                    _ => {}
                }
            }
        }
    });

    if quit.get() {
        system.exit();
    }

    element! {
        View(width, justify_content: JustifyContent::Center) {
            View(width: Percent(75.0), flex_direction: FlexDirection::Column, padding_top: 1) {
                Text(content: format!("Progress: {}%", percent.get()), weight: Weight::Bold)
                SeekBar(percent: percent.get(), on_seek: move |value| percent.set(value))
                Text(content: "Click the bar or use Left/Right. q to quit.", weight: Weight::Light)
            }
        }
    }
}

fn main() {
    smol::block_on(element!(Example).fullscreen()).unwrap();
}
