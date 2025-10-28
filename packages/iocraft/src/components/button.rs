use crate::{
    component, components::View, element, hooks::UseTerminalEvents, AnyElement,
    FullscreenMouseEvent, HandlerMut, Hooks, KeyCode, KeyEvent, KeyEventKind, MouseEventKind,
    Props, TerminalEvent,
};

/// The props which can be passed to the [`Button`] component.
#[non_exhaustive]
#[derive(Default, Props)]
pub struct ButtonProps<'a> {
    /// The children of the component. Exactly one child is expected.
    pub children: Vec<AnyElement<'a>>,

    /// The handler to invoke when the button is triggered.
    ///
    /// The button can be triggered two ways:
    ///
    /// - By clicking on it with the mouse while in fullscreen mode.
    /// - By pressing the Enter or Space key while [`has_focus`](Self::has_focus) is `true`.
    pub handler: HandlerMut<'static, ()>,

    /// True if the button has focus and should process keyboard input.
    pub has_focus: bool,
}

/// `Button` is a component that invokes a handler when clicked or when the Enter or Space key is pressed while it has focus.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # fn foo() -> impl Into<AnyElement<'static>> {
/// element! {
///     Button(handler: |_| { /* do something */ }, has_focus: true) {
///         View(border_style: BorderStyle::Round, border_color: Color::Blue) {
///             Text(content: "Click me!")
///         }
///     }
/// }
/// # }
/// ```
#[component]
pub fn Button<'a>(mut hooks: Hooks, props: &mut ButtonProps<'a>) -> impl Into<AnyElement<'a>> {
    hooks.use_local_terminal_events({
        let mut handler = props.handler.take();
        let has_focus = props.has_focus;
        move |event| match event {
            TerminalEvent::FullscreenMouse(FullscreenMouseEvent {
                kind: MouseEventKind::Down(_),
                ..
            }) => {
                handler(());
            }
            TerminalEvent::Key(KeyEvent { code, kind, .. })
                if has_focus
                    && kind != KeyEventKind::Release
                    && (code == KeyCode::Enter || code == KeyCode::Char(' ')) =>
            {
                handler(());
            }
            _ => {}
        }
    });

    match props.children.iter_mut().next() {
        Some(child) => child.into(),
        None => element!(View).into_any(),
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crossterm::event::MouseButton;
    use futures::stream::StreamExt;
    use macro_rules_attribute::apply;
    use smol_macros::test;

    #[component]
    fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut should_exit = hooks.use_state(|| false);

        if should_exit.get() {
            system.exit();
        }

        element! {
            Button(handler: move |_| should_exit.set(true), has_focus: true) {
                Text(content: "Exit")
            }
        }
    }

    #[apply(test!)]
    async fn test_button_click() {
        let actual = element!(MyComponent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::once(
                async {
                    TerminalEvent::FullscreenMouse(FullscreenMouseEvent::new(
                        MouseEventKind::Down(MouseButton::Left),
                        2,
                        0,
                    ))
                },
            )))
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .await;
        let expected = vec!["Exit\n"];
        assert_eq!(actual, expected);
    }

    #[apply(test!)]
    async fn test_button_key_input() {
        let actual = element!(MyComponent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::once(
                async { TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Enter)) },
            )))
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .await;
        let expected = vec!["Exit\n"];
        assert_eq!(actual, expected);
    }
}
