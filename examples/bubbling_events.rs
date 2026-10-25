use iocraft::prelude::*;

struct Change {
    amount: i32,
    consume: bool,
}

#[component]
fn Controls(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut handled = hooks.use_state(|| 0);
    let mut should_exit = hooks.use_state(|| false);
    let dispatcher = hooks.use_event_dispatcher::<Change>();
    hooks.use_events::<Change>(move |event| {
        handled += 1;
        if event.data.consume {
            event.stop_propagation();
        }
    });
    hooks.use_terminal_events(move |event| {
        if let TerminalEvent::Key(KeyEvent { code, kind, .. }) = event {
            if kind != KeyEventKind::Release {
                match code {
                    KeyCode::Char('+') => dispatcher.dispatch(Change {
                        amount: 1,
                        consume: false,
                    }),
                    KeyCode::Char('-') => dispatcher.dispatch(Change {
                        amount: -1,
                        consume: false,
                    }),
                    KeyCode::Char('x') => dispatcher.dispatch(Change {
                        amount: 1,
                        consume: true,
                    }),
                    KeyCode::Char('q') => should_exit.set(true),
                    _ => {}
                }
            }
        }
    });
    if should_exit.get() {
        hooks.use_context_mut::<SystemContext>().exit();
    }
    element!(Text(content: format!("Local events: {}", handled)))
}

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut total = hooks.use_state(|| 0);
    hooks.use_events::<Change>(move |event| {
        total += event.data.amount;
    });
    element! {
        View(flex_direction: FlexDirection::Column, padding: 1) {
            Text(content: format!("Bubbled total: {}", total), weight: Weight::Bold)
            Controls
            Text(content: "+/- bubble to the parent; x is consumed locally; q quits.")
        }
    }
}

fn main() {
    smol::block_on(element!(App).render_loop()).unwrap();
}
