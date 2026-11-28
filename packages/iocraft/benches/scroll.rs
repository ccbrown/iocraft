mod support;

use criterion::{criterion_group, criterion_main, Criterion};
use iocraft::prelude::*;
use support::{measure_repaint_sequence, measure_repaints, LOREM};

fn scroll_view(auto_scroll: bool) -> impl ElementExt {
    let paragraph = LOREM.repeat(4);
    element! {
        View(width: 120, height: 40) {
            ScrollView(auto_scroll) {
                View(width: 100pct, flex_direction: FlexDirection::Column) {
                    #((0..400).map(|row| element! {
                        View(key: row) {
                            Text(content: format!("{row} {paragraph}"), wrap: TextWrap::Wrap)
                        }
                    }))
                }
            }
        }
    }
}

fn wheel(kind: MouseEventKind) -> TerminalEvent {
    TerminalEvent::FullscreenMouse(FullscreenMouseEvent::new(kind, 10, 10))
}

fn benchmark(c: &mut Criterion) {
    c.bench_function("scroll/400", |b| {
        let down = wheel(MouseEventKind::ScrollDown);
        b.iter_custom(|iterations| {
            (0..iterations)
                .map(|_| measure_repaints(&mut scroll_view(false), down.clone(), 20))
                .sum()
        });
    });
    c.bench_function("scroll/auto-scroll-bottom/400", |b| {
        let pair = [
            wheel(MouseEventKind::ScrollUp),
            wheel(MouseEventKind::ScrollDown),
        ];
        b.iter_custom(|iterations| {
            let inputs = (0..iterations).flat_map(|_| pair.clone());
            measure_repaint_sequence(&mut scroll_view(true), inputs)
        });
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
