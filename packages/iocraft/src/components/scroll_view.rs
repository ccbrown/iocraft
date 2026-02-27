use crate::{
    component,
    components::View,
    element,
    hooks::{Ref, State, UseRef, UseState, UseTerminalEvents},
    AnyElement, CanvasTextStyle, Color, Component, ComponentDrawer, ComponentUpdater,
    FlexDirection, Hook, Hooks, JustifyContent, KeyCode, KeyEvent, KeyEventKind, MouseEventKind,
    Overflow, Position, Props, TerminalEvent,
};

/// A handle which can be used for imperative control of a [`ScrollView`] component.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # #[component]
/// # fn MyScrollable(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
/// let handle = hooks.use_ref_default::<ScrollViewHandle>();
///
/// hooks.use_terminal_events({
///     let mut handle = handle;
///     move |event| {
///         if let TerminalEvent::Key(KeyEvent { code: KeyCode::Home, kind: KeyEventKind::Press, .. }) = event {
///             handle.write().scroll_to_top();
///         }
///     }
/// });
///
/// element! {
///     View(width: 80, height: 20) {
///         ScrollView(handle) {
///             Text(content: "lots of content here...")
///         }
///     }
/// }
/// # }
/// ```
#[derive(Default)]
pub struct ScrollViewHandle {
    inner: Option<ScrollViewHandleInner>,
}

struct ScrollViewHandleInner {
    scroll_offset: State<i32>,
    content_height: State<u16>,
    viewport_height: State<u16>,
}

impl ScrollViewHandle {
    /// Scrolls to the top of the content.
    pub fn scroll_to_top(&mut self) {
        if let Some(inner) = &mut self.inner {
            inner.scroll_offset.set(0);
        }
    }

    /// Scrolls to the bottom of the content.
    pub fn scroll_to_bottom(&mut self) {
        if let Some(inner) = &mut self.inner {
            let max = max_offset(inner.content_height.get(), inner.viewport_height.get());
            inner.scroll_offset.set(max);
        }
    }

    /// Scrolls to the given offset in lines from the top. The offset is clamped to the valid
    /// range.
    pub fn scroll_to(&mut self, offset: i32) {
        if let Some(inner) = &mut self.inner {
            inner.scroll_offset.set(clamp_offset(
                offset,
                inner.content_height.get(),
                inner.viewport_height.get(),
            ));
        }
    }

    /// Scrolls by the given number of lines (positive = down, negative = up). The resulting
    /// offset is clamped to the valid range.
    pub fn scroll_by(&mut self, delta: i32) {
        if let Some(inner) = &mut self.inner {
            inner.scroll_offset.set(clamp_offset(
                inner.scroll_offset.get() + delta,
                inner.content_height.get(),
                inner.viewport_height.get(),
            ));
        }
    }

    /// Returns the current scroll offset in lines from the top.
    pub fn scroll_offset(&self) -> i32 {
        self.inner
            .as_ref()
            .map_or(0, |inner| inner.scroll_offset.get())
    }

    /// Returns the total height of the scrollable content in lines.
    pub fn content_height(&self) -> u16 {
        self.inner
            .as_ref()
            .map_or(0, |inner| inner.content_height.get())
    }

    /// Returns the height of the visible viewport in lines.
    pub fn viewport_height(&self) -> u16 {
        self.inner
            .as_ref()
            .map_or(0, |inner| inner.viewport_height.get())
    }
}

fn max_offset(content_height: u16, viewport_height: u16) -> i32 {
    (content_height as i32 - viewport_height as i32).max(0)
}

fn clamp_offset(offset: i32, content_height: u16, viewport_height: u16) -> i32 {
    offset.clamp(0, max_offset(content_height, viewport_height))
}

const DEFAULT_SCROLL_STEP: u16 = 3;

// -- Scrollbar component --

#[derive(Default, Props)]
struct ScrollViewScrollbarProps {
    viewport_height: u16,
    content_height: u16,
    scroll_offset: i32,
    thumb_color: Option<Color>,
    track_color: Option<Color>,
}

#[derive(Default)]
struct ScrollViewScrollbar {
    viewport_height: u16,
    content_height: u16,
    scroll_offset: i32,
    thumb_color: Option<Color>,
    track_color: Option<Color>,
}

impl Component for ScrollViewScrollbar {
    type Props<'a> = ScrollViewScrollbarProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self::default()
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        self.viewport_height = props.viewport_height;
        self.content_height = props.content_height;
        self.scroll_offset = props.scroll_offset;
        self.thumb_color = props.thumb_color;
        self.track_color = props.track_color;

        updater.set_layout_style(taffy::style::Style {
            size: taffy::geometry::Size {
                width: taffy::style::Dimension::Length(1.0),
                height: taffy::style::Dimension::Percent(1.0),
            },
            ..Default::default()
        });
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_>) {
        let vh = self.viewport_height as usize;
        let ch = self.content_height as usize;
        if vh == 0 || ch <= vh {
            return;
        }

        let thumb_size = (vh * vh / ch).max(1);
        let max_off = (ch - vh) as i32;
        let thumb_pos = if max_off > 0 {
            (self.scroll_offset as usize * (vh - thumb_size)) / max_off as usize
        } else {
            0
        };

        let thumb_color = self.thumb_color.unwrap_or(Color::White);
        let track_color = self.track_color.unwrap_or(Color::DarkGrey);
        let track_style = CanvasTextStyle {
            color: Some(track_color),
            ..Default::default()
        };
        let thumb_style = CanvasTextStyle {
            color: Some(thumb_color),
            ..Default::default()
        };

        let mut canvas = drawer.canvas();
        for y in 0..vh {
            if y >= thumb_pos && y < thumb_pos + thumb_size {
                canvas.set_text(0, y as isize, "\u{2503}", thumb_style); // ┃
            } else {
                canvas.set_text(0, y as isize, "\u{2502}", track_style); // │
            }
        }
    }
}

/// The props which can be passed to the [`ScrollView`] component.
#[non_exhaustive]
#[derive(Default, Props)]
pub struct ScrollViewProps<'a> {
    /// The children to render inside the scroll view.
    pub children: Vec<AnyElement<'a>>,
    /// When true, the scroll view stays pinned to the bottom as content grows.
    /// Scrolling up disengages auto scroll; reaching the bottom re-engages it.
    pub auto_scroll: bool,
    /// Number of lines to scroll per mouse wheel tick. Defaults to 3.
    pub scroll_step: Option<u16>,
    /// An optional handle which can be used for imperative control of the scroll view.
    pub handle: Option<Ref<ScrollViewHandle>>,
    /// Whether to show a scrollbar. Defaults to `true`.
    pub scrollbar: Option<bool>,
    /// Optional color for the scrollbar thumb. Defaults to `White`.
    pub scrollbar_thumb_color: Option<Color>,
    /// Optional color for the scrollbar track. Defaults to `DarkGrey`.
    pub scrollbar_track_color: Option<Color>,
}

// Hook that measures the component height in pre_component_draw and writes
// the result to a State<u16>.
struct MeasureHeightHook {
    out: State<u16>,
}

impl Hook for MeasureHeightHook {
    fn pre_component_draw(&mut self, drawer: &mut ComponentDrawer) {
        let h = drawer.size().height;
        if self.out.try_get() != Some(h) {
            self.out.set(h);
        }
    }
}

/// `ScrollView` is a component that provides scrollable content with keyboard and mouse support.
///
/// Place it inside a container with a fixed height. The scroll view will clip its children and
/// allow scrolling through them using arrow keys, Page Up/Down, Home/End, and mouse wheel.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # #[component]
/// # fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
/// element! {
///     View(width: 80, height: 20, border_style: BorderStyle::Round) {
///         ScrollView {
///             Text(content: "Line 1\nLine 2\nLine 3\n...")
///         }
///     }
/// }
/// # }
/// ```
#[component]
pub fn ScrollView<'a>(
    mut hooks: Hooks,
    props: &mut ScrollViewProps<'a>,
) -> impl Into<AnyElement<'a>> {
    let mut scroll_offset = hooks.use_state(|| 0i32);
    let mut user_scrolled_up = hooks.use_state(|| false);
    let mut content_height: State<u16> = hooks.use_state(|| 0u16);
    let viewport_height: State<u16> = hooks.use_state(|| 0u16);
    let content_height_ref: Ref<u16> = hooks.use_ref(|| 0u16);

    // Measure the viewport (this component's) height.
    let h = hooks.use_hook(move || MeasureHeightHook {
        out: viewport_height,
    });
    h.out = viewport_height;

    let scroll_step = props.scroll_step.unwrap_or(DEFAULT_SCROLL_STEP) as i32;
    let auto_scroll = props.auto_scroll;

    // Sync content height from the ref written by the measurer child.
    let ch = content_height_ref.get();
    if content_height.get() != ch {
        content_height.set(ch);
    }

    // Wire up the handle.
    if let Some(handle_ref) = props.handle.as_mut() {
        handle_ref.set(ScrollViewHandle {
            inner: Some(ScrollViewHandleInner {
                scroll_offset,
                content_height,
                viewport_height,
            }),
        });
    }

    // Determine if we should use auto_scroll (pinned to bottom) mode.
    let pinned_to_bottom = auto_scroll && !user_scrolled_up.get();

    // When not pinned to bottom, clamp scroll offset.
    if !pinned_to_bottom {
        let clamped = clamp_offset(
            scroll_offset.get(),
            content_height.get(),
            viewport_height.get(),
        );
        if scroll_offset.get() != clamped {
            scroll_offset.set(clamped);
        }
    }

    hooks.use_terminal_events({
        let vh = viewport_height;
        move |event| {
            let delta = match &event {
                TerminalEvent::Key(KeyEvent { code, kind, .. })
                    if *kind != KeyEventKind::Release =>
                {
                    match code {
                        KeyCode::Up => Some(-1),
                        KeyCode::Down => Some(1),
                        KeyCode::PageUp => Some(-(vh.get() as i32).max(1)),
                        KeyCode::PageDown => Some((vh.get() as i32).max(1)),
                        KeyCode::Home => Some(i32::MIN / 2),
                        KeyCode::End => Some(i32::MAX / 2),
                        _ => None,
                    }
                }
                TerminalEvent::FullscreenMouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => Some(-scroll_step),
                    MouseEventKind::ScrollDown => Some(scroll_step),
                    _ => None,
                },
                _ => None,
            };

            if let Some(delta) = delta {
                let new_offset =
                    clamp_offset(scroll_offset.get() + delta, content_height.get(), vh.get());
                scroll_offset.set(new_offset);

                if auto_scroll {
                    let max = max_offset(content_height.get(), vh.get());
                    if delta < 0 {
                        user_scrolled_up.set(true);
                    } else if new_offset >= max {
                        user_scrolled_up.set(false);
                    }
                }
            }
        }
    });

    let children = std::mem::take(&mut props.children);
    let show_scrollbar =
        props.scrollbar.unwrap_or(true) && content_height.get() > viewport_height.get();
    let scrollbar_thumb_color = props.scrollbar_thumb_color;
    let scrollbar_track_color = props.scrollbar_track_color;

    let effective_offset = if pinned_to_bottom {
        max_offset(content_height.get(), viewport_height.get())
    } else {
        scroll_offset.get()
    };

    if pinned_to_bottom {
        if show_scrollbar {
            element! {
                View(width: 100pct, height: 100pct, flex_direction: FlexDirection::Row) {
                    View(
                        overflow: Overflow::Hidden,
                        flex_grow: 1.0,
                        height: 100pct,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexEnd,
                    ) {
                        ScrollViewContentMeasurer(
                            content_height_ref: Some(content_height_ref),
                        ) {
                            #(children)
                        }
                    }
                    ScrollViewScrollbar(
                        viewport_height: viewport_height.get(),
                        content_height: content_height.get(),
                        scroll_offset: effective_offset,
                        thumb_color: scrollbar_thumb_color,
                        track_color: scrollbar_track_color,
                    )
                }
            }
        } else {
            element! {
                View(
                    overflow: Overflow::Hidden,
                    width: 100pct,
                    height: 100pct,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                ) {
                    ScrollViewContentMeasurer(
                        content_height_ref: Some(content_height_ref),
                    ) {
                        #(children)
                    }
                }
            }
        }
    } else if show_scrollbar {
        element! {
            View(width: 100pct, height: 100pct, flex_direction: FlexDirection::Row) {
                View(overflow: Overflow::Hidden, flex_grow: 1.0, height: 100pct) {
                    View(position: Position::Absolute, top: -scroll_offset.get(), width: 100pct) {
                        ScrollViewContentMeasurer(
                            content_height_ref: Some(content_height_ref),
                        ) {
                            #(children)
                        }
                    }
                }
                ScrollViewScrollbar(
                    viewport_height: viewport_height.get(),
                    content_height: content_height.get(),
                    scroll_offset: effective_offset,
                    thumb_color: scrollbar_thumb_color,
                    track_color: scrollbar_track_color,
                )
            }
        }
    } else {
        element! {
            View(overflow: Overflow::Hidden, width: 100pct, height: 100pct) {
                View(position: Position::Absolute, top: -scroll_offset.get(), width: 100pct) {
                    ScrollViewContentMeasurer(
                        content_height_ref: Some(content_height_ref),
                    ) {
                        #(children)
                    }
                }
            }
        }
    }
}

#[derive(Default, Props)]
struct ScrollViewContentMeasurerProps<'a> {
    children: Vec<AnyElement<'a>>,
    content_height_ref: Option<Ref<u16>>,
}

// Hook that measures this component's height and writes it to a Ref<u16>
// shared with the parent ScrollView.
struct ContentHeightHook {
    out: Option<Ref<u16>>,
}

impl Hook for ContentHeightHook {
    fn pre_component_draw(&mut self, drawer: &mut ComponentDrawer) {
        if let Some(mut out) = self.out {
            let h = drawer.size().height;
            if out.try_get() != Some(h) {
                out.set(h);
            }
        }
    }
}

/// Private component that lives inside the scroll pane so that
/// its `pre_component_draw` size equals the natural content height.
#[component]
fn ScrollViewContentMeasurer<'a>(
    mut hooks: Hooks,
    props: &mut ScrollViewContentMeasurerProps<'a>,
) -> impl Into<AnyElement<'a>> {
    let content_height_ref = props.content_height_ref;
    let h = hooks.use_hook(move || ContentHeightHook {
        out: content_height_ref,
    });
    h.out = content_height_ref;

    let children = std::mem::take(&mut props.children);
    element! {
        View(width: 100pct) {
            #(children)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use futures::stream::{self, StreamExt};
    use macro_rules_attribute::apply;
    use smol_macros::test;

    #[component]
    fn TestScrollView(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut done = hooks.use_state(|| false);

        hooks.use_terminal_events(move |event| {
            if let TerminalEvent::Key(KeyEvent {
                code: KeyCode::Char('q'),
                kind: KeyEventKind::Press,
                ..
            }) = event
            {
                done.set(true);
            }
        });

        if done.get() {
            system.exit();
        }

        let mut lines = String::new();
        for i in 0..20 {
            if i > 0 {
                lines.push('\n');
            }
            lines.push_str(&format!("Line {i}"));
        }

        element! {
            View(width: 20, height: 5) {
                ScrollView {
                    Text(content: lines)
                }
            }
        }
    }

    #[apply(test!)]
    async fn test_scroll_view_basic_render() {
        let canvases: Vec<_> = element!(TestScrollView)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(stream::iter(vec![
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('q'))),
            ])))
            .collect()
            .await;

        let output = canvases.last().unwrap().to_string();
        assert!(output.contains("Line 0"));
        assert!(output.contains("Line 4"));
        assert!(!output.contains("Line 5"));
    }

    #[apply(test!)]
    async fn test_scroll_view_keyboard_scroll() {
        let canvases: Vec<_> = element!(TestScrollView)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(stream::iter(vec![
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Down)),
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Down)),
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('q'))),
            ])))
            .collect()
            .await;

        let output = canvases.last().unwrap().to_string();
        assert!(output.contains("Line 2"));
        assert!(!output.contains("Line 0"));
    }

    #[apply(test!)]
    async fn test_scroll_view_content_shorter_than_viewport() {
        #[component]
        fn ShortContent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let mut system = hooks.use_context_mut::<SystemContext>();
            let mut done = hooks.use_state(|| false);

            hooks.use_terminal_events(move |event| {
                if let TerminalEvent::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    kind: KeyEventKind::Press,
                    ..
                }) = event
                {
                    done.set(true);
                }
            });

            if done.get() {
                system.exit();
            }

            element! {
                View(width: 20, height: 10) {
                    ScrollView {
                        Text(content: "Short")
                    }
                }
            }
        }

        let canvases: Vec<_> = element!(ShortContent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(stream::iter(vec![
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Down)),
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('q'))),
            ])))
            .collect()
            .await;

        let output = canvases.last().unwrap().to_string();
        assert!(output.contains("Short"));
    }

    #[apply(test!)]
    async fn test_scroll_view_auto_scroll() {
        #[component]
        fn AutoScrollContent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let mut system = hooks.use_context_mut::<SystemContext>();
            let mut done = hooks.use_state(|| false);

            hooks.use_terminal_events(move |event| {
                if let TerminalEvent::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    kind: KeyEventKind::Press,
                    ..
                }) = event
                {
                    done.set(true);
                }
            });

            if done.get() {
                system.exit();
            }

            let mut lines = String::new();
            for i in 0..20 {
                if i > 0 {
                    lines.push('\n');
                }
                lines.push_str(&format!("Line {i}"));
            }

            element! {
                View(width: 20, height: 5) {
                    ScrollView(auto_scroll: true) {
                        Text(content: lines)
                    }
                }
            }
        }

        let canvases: Vec<_> = element!(AutoScrollContent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(stream::iter(vec![
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('q'))),
            ])))
            .collect()
            .await;

        let output = canvases.last().unwrap().to_string();
        // With auto_scroll and content exceeding viewport, should see the last lines.
        assert!(output.contains("Line 19"));
        assert!(!output.contains("Line 0"));
    }

    #[apply(test!)]
    async fn test_scroll_view_shows_scrollbar() {
        let canvases: Vec<_> = element!(TestScrollView)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(stream::iter(vec![
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('q'))),
            ])))
            .collect()
            .await;

        let output = canvases.last().unwrap().to_string();
        // Scrollbar track character should be present when content exceeds viewport.
        assert!(output.contains('\u{2502}')); // │
    }

    #[apply(test!)]
    async fn test_scroll_view_no_scrollbar_when_disabled() {
        #[component]
        fn NoScrollbar(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
            let mut system = hooks.use_context_mut::<SystemContext>();
            let mut done = hooks.use_state(|| false);

            hooks.use_terminal_events(move |event| {
                if let TerminalEvent::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    kind: KeyEventKind::Press,
                    ..
                }) = event
                {
                    done.set(true);
                }
            });

            if done.get() {
                system.exit();
            }

            let mut lines = String::new();
            for i in 0..20 {
                if i > 0 {
                    lines.push('\n');
                }
                lines.push_str(&format!("Line {i}"));
            }

            element! {
                View(width: 20, height: 5) {
                    ScrollView(scrollbar: Some(false)) {
                        Text(content: lines)
                    }
                }
            }
        }

        let canvases: Vec<_> = element!(NoScrollbar)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(stream::iter(vec![
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('q'))),
            ])))
            .collect()
            .await;

        let output = canvases.last().unwrap().to_string();
        // Scrollbar characters should not be present.
        assert!(!output.contains('\u{2502}')); // │
        assert!(!output.contains('\u{2503}')); // ┃
    }
}
