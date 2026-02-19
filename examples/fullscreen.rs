use chrono::Local;
use iocraft::prelude::*;
use std::{backtrace::Backtrace, cell::Cell, time::Duration};

thread_local! {
    static BACKTRACE: Cell<Option<Backtrace>> = const { Cell::new(None) };
}

#[component]
fn Example(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let (width, height) = hooks.use_terminal_size();
    let mut system = hooks.use_context_mut::<SystemContext>();
    let mut time = hooks.use_state(|| Local::now());
    let mut should_exit = hooks.use_state(|| false);

    hooks.use_future(async move {
        loop {
            smol::Timer::after(Duration::from_secs(1)).await;
            time.set(Local::now());
        }
    });

    hooks.use_terminal_events({
        move |event| match event {
            TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
                match code {
                    KeyCode::Char('q') => should_exit.set(true),
                    KeyCode::Char('p') => panic!("oh, no!"),
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
            width,
            height,
            background_color: Color::DarkGrey,
            border_style: BorderStyle::Double,
            border_color: Color::Blue,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        ) {
            View(
                border_style: BorderStyle::Round,
                border_color: Color::Blue,
                margin_bottom: 2,
                padding_top: 2,
                padding_bottom: 2,
                padding_left: 8,
                padding_right: 8,
            ) {
                Text(content: format!("Current Time: {}", time.get().format("%r")))
            }
            Text(content: "Press \"q\" to quit. \"p\" to panic")
        }
    }
}

fn main() {
    // try to add some panic!() somewhere in the component for test
    // when the panic is triggered, we will restore the original terminal
    // so that the panic info and backtrace can be correctly shown
    std::panic::set_hook(Box::new(|_| {
        let trace = Backtrace::capture();
        BACKTRACE.with(move |b| b.set(Some(trace)));
    }));

    if std::panic::catch_unwind(|| {
        smol::block_on(element!(Example).fullscreen()).unwrap();
    })
    .is_err()
    {
        let b = BACKTRACE.with(|b| b.take()).unwrap();
        println!("panic:\n\n{b}");
    }
}
