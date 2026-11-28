mod support;

use criterion::{criterion_group, criterion_main, Criterion};
use iocraft::prelude::*;
use support::{measure_repaints, LOREM};

const TEXT_BYTES: usize = 100_000;

#[derive(Default, Props)]
struct StyledTextProps {
    content: String,
}

#[component]
fn StyledText(props: &StyledTextProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut alternate = hooks.use_state(|| false);
    hooks.use_terminal_events(move |_| alternate.set(!alternate.get()));

    element! {
        View(width: 120) {
            Text(
                content: props.content.clone(),
                color: if alternate.get() { Color::Red } else { Color::Blue },
                wrap: TextWrap::Wrap,
            )
        }
    }
}

fn benchmark(c: &mut Criterion) {
    let mut content = LOREM.repeat(TEXT_BYTES.div_ceil(LOREM.len()));
    content.truncate(TEXT_BYTES);
    let key = TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('c')));

    c.bench_function("text/style-only", |b| {
        b.iter_custom(|iterations| {
            (0..iterations)
                .map(|_| {
                    let mut app = element!(StyledText(content: content.clone()));
                    measure_repaints(&mut app, key.clone(), 20)
                })
                .sum()
        });
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
