use criterion::{criterion_group, criterion_main, Criterion};
use futures::{channel::mpsc::unbounded, FutureExt, StreamExt};
use iocraft::prelude::*;
use std::{
    pin::pin,
    time::{Duration, Instant},
};

const LOREM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ";

fn scroll() -> Duration {
    let paragraph = LOREM.repeat(4);
    let mut app = element! {
        View(width: 120, height: 40) {
            ScrollView {
                View(width: 100pct, flex_direction: FlexDirection::Column) {
                    #((0..400).map(|row| element! {
                        View(key: row) {
                            Text(content: format!("{row} {paragraph}"), wrap: TextWrap::Wrap)
                        }
                    }))
                }
            }
        }
    };

    smol::block_on(async {
        let (input, events) = unbounded();
        let mut frames =
            pin!(app.mock_terminal_render_loop(MockTerminalConfig::with_events(events)));
        frames.next().await.expect("initial frame");
        // ScrollView can repaint while settling its initial dimensions.
        while frames.next().now_or_never().flatten().is_some() {}
        let wheel = TerminalEvent::FullscreenMouse(FullscreenMouseEvent::new(
            MouseEventKind::ScrollDown,
            10,
            10,
        ));
        let start = Instant::now();
        // The render loop drains queued input before repainting.
        for _ in 0..20 {
            input.unbounded_send(wheel.clone()).unwrap();
            frames.next().await.expect("scroll frame");
        }
        start.elapsed()
    })
}

fn benchmark(c: &mut Criterion) {
    c.bench_function("scroll/400", |b| {
        b.iter_custom(|iterations| (0..iterations).map(|_| scroll()).sum());
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
