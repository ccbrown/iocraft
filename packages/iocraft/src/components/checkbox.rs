use crate::{
    component,
    components::{Button, Text, TextWrap, View},
    element, AnyElement, FlexDirection, HandlerMut, Hooks, Props,
};

/// The props which can be passed to the [`Checkbox`] component.
#[non_exhaustive]
#[derive(Default, Props)]
pub struct CheckboxProps<'a> {
    /// The elements to render after the checkbox indicator.
    pub children: Vec<AnyElement<'a>>,

    /// Whether the checkbox is checked.
    pub checked: bool,

    /// The handler to invoke with the new checked state when the checkbox is triggered.
    ///
    /// The checkbox can be triggered by clicking it with the mouse while in fullscreen mode, or by
    /// pressing Enter or Space while [`has_focus`](Self::has_focus) is `true`.
    pub on_change: HandlerMut<'static, bool>,

    /// True if the checkbox has focus and should process keyboard input.
    pub has_focus: bool,
}

/// `Checkbox` is a controlled component for toggling a boolean value.
///
/// The current value is provided through [`CheckboxProps::checked`]. When the checkbox is
/// triggered, [`CheckboxProps::on_change`] is called with the opposite value.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// #[component]
/// fn Example(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
///     let mut checked = hooks.use_state(|| false);
///
///     element! {
///         Checkbox(
///             checked: checked.get(),
///             on_change: move |new_value| checked.set(new_value),
///             has_focus: true,
///         ) {
///             Text(content: "Enable notifications")
///         }
///     }
/// }
/// ```
#[component]
pub fn Checkbox<'a>(_hooks: Hooks, props: &mut CheckboxProps<'a>) -> impl Into<AnyElement<'a>> {
    let checked = props.checked;
    let has_focus = props.has_focus;
    let mut on_change = props.on_change.take();
    let children = std::mem::take(&mut props.children);
    let separator = (!children.is_empty()).then(|| element!(Text(content: " ")));

    element! {
        Button(
            handler: move |_| on_change(!checked),
            has_focus,
        ) {
            View(flex_direction: FlexDirection::Row) {
                Text(
                    content: if checked { "[x]" } else { "[ ]" },
                    invert: has_focus,
                    wrap: TextWrap::NoWrap,
                )
                #(separator)
                #(children)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crossterm::event::MouseButton;
    use futures::stream::StreamExt;
    use macro_rules_attribute::apply;
    use smol_macros::test;

    #[derive(Default, Props)]
    struct TestCheckboxProps {
        has_focus: bool,
    }

    #[component]
    fn TestCheckbox(mut hooks: Hooks, props: &TestCheckboxProps) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut checked = hooks.use_state(|| false);

        if checked.get() {
            system.exit();
        }

        element! {
            Checkbox(
                checked: checked.get(),
                on_change: move |new_value| checked.set(new_value),
                has_focus: props.has_focus,
            ) {
                Text(content: "Enable feature")
            }
        }
    }

    #[component]
    fn UnfocusedCheckbox(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut checked = hooks.use_state(|| false);
        let mut event_received = hooks.use_state(|| false);

        hooks.use_terminal_events(move |_| event_received.set(true));
        if event_received.get() {
            system.exit();
        }

        element! {
            Checkbox(
                checked: checked.get(),
                on_change: move |new_value| checked.set(new_value),
            ) {
                Text(content: "Enable feature")
            }
        }
    }

    #[apply(test!)]
    async fn renders_and_toggles_with_keyboard() {
        let actual = element!(TestCheckbox(has_focus: true))
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter([
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char(' '))),
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char(' '))),
            ])))
            .map(|canvas| canvas.to_string())
            .collect::<Vec<_>>()
            .await;

        assert_eq!(actual, vec!["[ ] Enable feature\n", "[x] Enable feature\n"]);
    }

    #[apply(test!)]
    async fn toggles_with_mouse() {
        let actual = element!(TestCheckbox)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::once(
                async {
                    TerminalEvent::FullscreenMouse(FullscreenMouseEvent::new(
                        MouseEventKind::Down(MouseButton::Left),
                        1,
                        0,
                    ))
                },
            )))
            .map(|canvas| canvas.to_string())
            .collect::<Vec<_>>()
            .await;

        assert_eq!(actual, vec!["[ ] Enable feature\n", "[x] Enable feature\n"]);
    }

    #[apply(test!)]
    async fn ignores_keyboard_input_without_focus() {
        let actual = element!(UnfocusedCheckbox)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::once(
                async { TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Enter)) },
            )))
            .map(|canvas| canvas.to_string())
            .collect::<Vec<_>>()
            .await;

        assert_eq!(actual, vec!["[ ] Enable feature\n"]);
    }

    #[test]
    fn shows_focus_without_relying_on_color() {
        let canvas = element! {
            Checkbox(has_focus: true) {
                Text(content: "Label")
            }
        }
        .render(None);

        assert!(canvas.cell(0, 0).unwrap().text_style().unwrap().invert);
        assert!(!canvas.cell(4, 0).unwrap().text_style().unwrap().invert);
    }
}
