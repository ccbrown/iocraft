mod support;

use criterion::{criterion_group, criterion_main, Criterion};
use iocraft::prelude::*;
use std::time::Duration;
use support::{measure_repaints, LOREM};

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

    let wheel = TerminalEvent::FullscreenMouse(FullscreenMouseEvent::new(
        MouseEventKind::ScrollDown,
        10,
        10,
    ));
    measure_repaints(&mut app, wheel, 20)
}

fn benchmark(c: &mut Criterion) {
    c.bench_function("scroll/400", |b| {
        b.iter_custom(|iterations| (0..iterations).map(|_| scroll()).sum());
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
