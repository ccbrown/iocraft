use futures::{channel::mpsc::unbounded, FutureExt, StreamExt};
use iocraft::prelude::*;
use std::{
    pin::pin,
    time::{Duration, Instant},
};

pub const LOREM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ";

pub fn measure_repaints(app: &mut impl ElementExt, event: TerminalEvent, count: usize) -> Duration {
    smol::block_on(async {
        let (input, events) = unbounded();
        let mut frames =
            pin!(app.mock_terminal_render_loop(MockTerminalConfig::with_events(events)));
        frames.next().await.expect("initial frame");
        // Components can repaint while settling their initial dimensions.
        while frames.next().now_or_never().flatten().is_some() {}

        let start = Instant::now();
        // The render loop drains queued input before repainting.
        for _ in 0..count {
            input.unbounded_send(event.clone()).unwrap();
            frames.next().await.expect("repaint after input");
        }
        start.elapsed()
    })
}
