use iocraft::prelude::*;
use unicode_width::UnicodeWidthStr;

#[context]
struct ExampleContext<'a> {
    system: &'a mut SystemContext,
}

const AREA_WIDTH: u32 = 80;
const AREA_HEIGHT: u32 = 11;
const FACE: &str = "👾";

#[component]
fn Example(context: ExampleContext, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let x = hooks.use_state(|| 0);
    let y = hooks.use_state(|| 0);
    let should_exit = hooks.use_state(|| false);

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
        context.system.exit();
    }

    element! {
        Box(
            flex_direction: FlexDirection::Column,
            padding: 2,
            align_items: AlignItems::Center
        ) {
            Text(content: "Use arrow keys to move. Press \"q\" to exit.")
            Box(
                border_style: BorderStyle::Round,
                border_color: Color::Green,
                height: AREA_HEIGHT + 2,
                width: AREA_WIDTH + 2,
            ) {
                #(if should_exit.get() {
                    element! {
                        Box(
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
                        Box(
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
