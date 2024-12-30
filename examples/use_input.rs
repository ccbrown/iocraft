use iocraft::prelude::*;
use unicode_width::UnicodeWidthStr;

const AREA_WIDTH: u32 = 80;
const AREA_HEIGHT: u32 = 11;
const FACE: &str = "👾";

#[component]
fn Example(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut system = hooks.use_context_mut::<SystemContext>();
    let mut x = hooks.use_state(|| 0);
    let mut y = hooks.use_state(|| 0);
    let mut should_exit = hooks.use_state(|| false);

    hooks.use_terminal_events({
        move |event| match event {
            TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
                match code {
                    KeyCode::Char('q') => should_exit.set(true),
                    KeyCode::Up => y.set((y.get() as i32 - 1).max(0) as _),
                    KeyCode::Down => y.set((y.get() + 1).min(AREA_HEIGHT - 1)),
                    KeyCode::Left => x.set((x.get() as i32 - 1).max(0) as _),
                    KeyCode::Right => x.set((x.get() + 1).min(AREA_WIDTH - FACE.width() as u32)),
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
            Text(content: "Use arrow keys to move. Press \"q\" to exit.")
            View(
                border_style: BorderStyle::Round,
                border_color: Color::Green,
                height: AREA_HEIGHT + 2,
                width: AREA_WIDTH + 2,
            ) {
                #(if should_exit.get() {
                    element! {
                        View(
                            width: 100pct,
                            height: 100pct,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                        ) {
                            Text(content: format!("Goodbye! {}", FACE))
                        }
                    }
                } else {
                    element! {
                        View(
                            padding_left: x.get(),
                            padding_top: y.get(),
                        ) {
                            Text(content: FACE)
                        }
                    }
                })
            }
        }
    }
}

fn main() {
    smol::block_on(element!(Example).render_loop()).unwrap();
}
