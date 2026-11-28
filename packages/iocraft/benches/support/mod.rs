use futures::{channel::mpsc::unbounded, FutureExt, StreamExt};
use iocraft::prelude::*;
use std::{
    pin::pin,
    time::{Duration, Instant},
};

pub const LOREM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ";

pub fn measure_repaints(app: &mut impl ElementExt, event: TerminalEvent, count: usize) -> Duration {
    measure_repaint_sequence(app, std::iter::repeat_n(event, count))
}

pub fn measure_repaint_sequence(
    app: &mut impl ElementExt,
    inputs: impl IntoIterator<Item = TerminalEvent>,
) -> Duration {
    smol::block_on(async {
        let (input, events) = unbounded();
        let mut frames =
            pin!(app.mock_terminal_render_loop(MockTerminalConfig::with_events(events)));
        frames.next().await.expect("initial frame");
        // Components can repaint while settling their initial dimensions.
        while frames.next().now_or_never().flatten().is_some() {}

        let start = Instant::now();
        // The render loop can coalesce queued input, and one input can trigger multiple frames.
        for event in inputs {
            input.unbounded_send(event).unwrap();
            frames.next().await.expect("repaint after input");
            while frames.next().now_or_never().flatten().is_some() {}
        }
        start.elapsed()
    })
}
