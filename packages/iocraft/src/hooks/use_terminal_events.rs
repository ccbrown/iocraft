use crate::{ComponentUpdater, FullscreenMouseEvent, Hook, Hooks, TerminalEvent, TerminalEvents};
use core::{
    pin::{pin, Pin},
    task::{Context, Poll},
};
use futures::stream::Stream;
use taffy::{Point, Size};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// `UseTerminalEvents` is a hook that allows you to listen for user input such as key strokes.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # use unicode_width::UnicodeWidthStr;
/// const AREA_WIDTH: u32 = 80;
/// const AREA_HEIGHT: u32 = 11;
/// const FACE: &str = "👾";
///
/// #[component]
/// fn Example(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
///     let mut system = hooks.use_context_mut::<SystemContext>();
///     let mut x = hooks.use_state(|| 0);
///     let mut y = hooks.use_state(|| 0);
///     let mut should_exit = hooks.use_state(|| false);
///
///     hooks.use_terminal_events({
///         move |event| match event {
///             TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
///                 match code {
///                     KeyCode::Char('q') => should_exit.set(true),
///                     KeyCode::Up => y.set((y.get() as i32 - 1).max(0) as _),
///                     KeyCode::Down => y.set((y.get() + 1).min(AREA_HEIGHT - 1)),
///                     KeyCode::Left => x.set((x.get() as i32 - 1).max(0) as _),
///                     KeyCode::Right => x.set((x.get() + 1).min(AREA_WIDTH - FACE.width() as u32)),
///                     _ => {}
///                 }
///             }
///             _ => {}
///         }
///     });
///
///     if should_exit.get() {
///         system.exit();
///     }
///
///     element! {
///         View(
///             flex_direction: FlexDirection::Column,
///             padding: 2,
///             align_items: AlignItems::Center
///         ) {
///             Text(content: "Use arrow keys to move. Press \"q\" to exit.")
///             View(
///                 border_style: BorderStyle::Round,
///                 border_color: Color::Green,
///                 height: AREA_HEIGHT + 2,
///                 width: AREA_WIDTH + 2,
///             ) {
///                 #(if should_exit.get() {
///                     element! {
///                         View(
///                             width: 100pct,
///                             height: 100pct,
///                             justify_content: JustifyContent::Center,
///                             align_items: AlignItems::Center,
///                         ) {
///                             Text(content: format!("Goodbye! {}", FACE))
///                         }
///                     }
///                 } else {
///                     element! {
///                         View(
///                             padding_left: x.get(),
///                             padding_top: y.get(),
///                         ) {
///                             Text(content: FACE)
///                         }
///                     }
///                 })
///             }
///         }
///     }
/// }
/// ```
pub trait UseTerminalEvents: private::Sealed {
    /// Defines a callback to be invoked whenever a terminal event occurs.
    ///
    /// This hook will be called for all terminal events, including those that occur outside of the
    /// component. If you only want to listen for events within the component, use
    /// [`Self::use_local_terminal_events`] instead.
    fn use_terminal_events<F>(&mut self, f: F)
    where
        F: FnMut(TerminalEvent) + Send + 'static;

    /// Defines a callback to be invoked whenever a terminal event occurs within a component.
    ///
    /// Unlike [`Self::use_terminal_events`], this hook will not be called for events such as mouse
    /// events that occur outside of the component. Furthermore, coordinates will be translated to
    /// component-local coordinates.
    fn use_local_terminal_events<F>(&mut self, f: F)
    where
        F: FnMut(TerminalEvent) + Send + 'static;
}

impl UseTerminalEvents for Hooks<'_, '_> {
    fn use_terminal_events<F>(&mut self, f: F)
    where
        F: FnMut(TerminalEvent) + Send + 'static,
    {
        self.use_hook(move || UseTerminalEventsImpl {
            events: None,
            component_location: Default::default(),
            in_component: false,
            f: Box::new(f),
        });
    }

    fn use_local_terminal_events<F>(&mut self, f: F)
    where
        F: FnMut(TerminalEvent) + Send + 'static,
    {
        self.use_hook(move || UseTerminalEventsImpl {
            events: None,
            component_location: Default::default(),
            in_component: true,
            f: Box::new(f),
        });
    }
}

struct UseTerminalEventsImpl {
    events: Option<TerminalEvents>,
    component_location: (Point<i16>, Size<u16>),
    in_component: bool,
    f: Box<dyn FnMut(TerminalEvent) + Send + 'static>,
}

impl Hook for UseTerminalEventsImpl {
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        while let Some(Poll::Ready(Some(event))) = self
            .events
            .as_mut()
            .map(|events| pin!(events).poll_next(cx))
        {
            if self.in_component {
                let (location, size) = self.component_location;
                match event {
                    TerminalEvent::FullscreenMouse(event) => {
                        if event.row as i16 >= location.y && event.column as i16 >= location.x {
                            let row = (event.row as i16 - location.y) as u16;
                            let column = (event.column as i16 - location.x) as u16;
                            if row < size.height && column < size.width {
                                (self.f)(TerminalEvent::FullscreenMouse(FullscreenMouseEvent {
                                    row,
                                    column,
                                    ..event
                                }));
                            }
                        }
                    }
                    TerminalEvent::Key(_) | TerminalEvent::Resize(..) => {
                        (self.f)(event);
                    }
                }
            } else {
                (self.f)(event);
            }
        }
        Poll::Pending
    }

    fn post_component_update(&mut self, updater: &mut ComponentUpdater) {
        if self.events.is_none() {
            self.events = updater.terminal_events();
        }
    }

    fn post_component_draw(&mut self, drawer: &mut crate::ComponentDrawer) {
        self.component_location = (drawer.canvas_position(), drawer.size());
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crossterm::event::MouseButton;
    use futures::stream::{self, StreamExt};
    use macro_rules_attribute::apply;
    use smol_macros::test;

    #[component]
    fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut should_exit = hooks.use_state(|| false);
        hooks.use_terminal_events(move |_event| {
            should_exit.set(true);
        });

        if should_exit.get() {
            system.exit();
            element!(Text(content:"received event")).into_any()
        } else {
            element!(View).into_any()
        }
    }

    #[apply(test!)]
    async fn test_use_terminal_events() {
        let canvases: Vec<_> = element!(MyComponent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(stream::iter(vec![
                TerminalEvent::Key(KeyEvent {
                    code: KeyCode::Char('f'),
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                }),
            ])))
            .collect()
            .await;
        let actual = canvases.iter().map(|c| c.to_string()).collect::<Vec<_>>();
        let expected = vec!["", "received event\n"];
        assert_eq!(actual, expected);
    }

    #[component]
    fn MyClickableComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut should_exit = hooks.use_state(|| false);
        hooks.use_local_terminal_events(move |event| {
            if let TerminalEvent::FullscreenMouse(FullscreenMouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                row,
                column,
                ..
            }) = event
            {
                assert_eq!(row, 8);
                assert_eq!(column, 8);
                should_exit.set(true);
            }
        });

        if should_exit.get() {
            system.exit();
            element!(Text(content:"received click")).into_any()
        } else {
            element!(View(width: 10, height: 10)).into_any()
        }
    }

    #[apply(test!)]
    async fn test_use_local_terminal_events() {
        let canvases: Vec<_> = element! {
            View(padding: 2) {
                MyClickableComponent
            }
        }
        .mock_terminal_render_loop(MockTerminalConfig::with_events(stream::iter(vec![
            TerminalEvent::FullscreenMouse(FullscreenMouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                10,
                10,
            )),
        ])))
        .collect()
        .await;
        let actual = canvases
            .iter()
            .map(|c| c.to_string().trim().to_string())
            .collect::<Vec<_>>();
        assert_eq!(actual, vec!["", "received click"]);
    }
}
