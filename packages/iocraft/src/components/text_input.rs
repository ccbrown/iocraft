use crate::{
    CanvasTextStyle, Color, Component, ComponentDrawer, ComponentUpdater, Handler, Hooks, KeyCode,
    KeyEvent, KeyEventKind, Props, TerminalEvent, TerminalEvents,
};
use futures::stream::Stream;
use std::{
    mem,
    pin::{pin, Pin},
    task::{Context, Poll},
};
use unicode_width::UnicodeWidthStr;

/// The props which can be passed to the [`TextInput`] component.
#[non_exhaustive]
#[derive(Default, Props)]
pub struct TextInputProps {
    /// The color to make the text.
    pub color: Option<Color>,

    /// The current value.
    pub value: String,

    /// True if the input has focus and should process keyboard input.
    pub has_focus: bool,

    /// The handler to invoke when the value changes.
    pub on_change: Handler<'static, String>,
}

/// `TextInput` is a component that can receive text input from the user.
///
/// It will fill the available space and display the current value. Typically, you will want to
/// render it in a [`Box`] component of the desired text field size.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # #[component]
/// # fn FormField(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
/// let value = hooks.use_state(|| "".to_string());
///
/// element! {
///     Box(
///         border_style: BorderStyle::Round,
///         border_color: Color::Blue,
///     ) {
///         Box(width: 15) {
///             Text(content: "Input: ")
///         }
///         Box(
///             background_color: Color::DarkGrey,
///             width: 30,
///         ) {
///             TextInput(
///                 has_focus: true,
///                 value: value.to_string(),
///                 on_change: move |new_value| value.set(new_value),
///             )
///         }
///     }
/// }
/// # }
/// ```
#[derive(Default)]
pub struct TextInput {
    value: String,
    events: Option<TerminalEvents>,
    style: CanvasTextStyle,
    handler: Handler<'static, String>,
    has_focus: bool,
}

impl Component for TextInput {
    type Props<'a> = TextInputProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self::default()
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        if self.events.is_none() {
            self.events = updater.terminal_events();
        }
        self.style = CanvasTextStyle {
            color: props.color,
            ..Default::default()
        };
        self.value = props.value.clone();
        self.handler = mem::take(&mut props.on_change);
        self.has_focus = props.has_focus;
        updater.set_layout_style(taffy::style::Style {
            size: taffy::Size::percent(1.0),
            ..Default::default()
        });
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_>) {
        let size = drawer.layout().size;

        let mut max_width = 0;
        let mut num_lines = 0;
        for line in self.value.lines() {
            max_width = max_width.max(line.width());
            num_lines += 1;
        }
        num_lines = num_lines.max(1);

        let y = if num_lines > size.height as usize {
            -(num_lines as isize - size.height as isize)
        } else {
            0
        };

        let x = if max_width > size.width as usize {
            -(max_width as isize - size.width as isize)
        } else {
            0
        };

        drawer.canvas().set_text(x, y, &self.value, self.style);
    }

    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut changed = false;
        while let Some(Poll::Ready(Some(event))) = self
            .events
            .as_mut()
            .map(|events| pin!(events).poll_next(cx))
        {
            if !self.has_focus {
                continue;
            }
            match event {
                TerminalEvent::Key(KeyEvent { code, kind, .. })
                    if kind != KeyEventKind::Release =>
                {
                    match code {
                        KeyCode::Char(c) => {
                            changed = true;
                            self.value.push(c);
                        }
                        KeyCode::Backspace => {
                            changed = true;
                            self.value.pop();
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if changed {
            let new_value = self.value.clone();
            (self.handler)(new_value);
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use futures::stream::StreamExt;
    use macro_rules_attribute::apply;
    use smol_macros::test;

    #[component]
    fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let value = hooks.use_state(|| "".to_string());

        if value.read().as_str() == "foo" {
            system.exit();
        }

        element! {
            Box(height: 1, width: 10) {
                TextInput(
                    has_focus: true,
                    value: value.to_string(),
                    on_change: move |new_value| value.set(new_value),
                )
            }
        }
    }

    #[apply(test!)]
    async fn test_text_input() {
        let actual = element!(MyComponent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                vec![
                    TerminalEvent::Key(KeyEvent {
                        code: KeyCode::Char('f'),
                        modifiers: KeyModifiers::empty(),
                        kind: KeyEventKind::Press,
                    }),
                    TerminalEvent::Key(KeyEvent {
                        code: KeyCode::Char('f'),
                        modifiers: KeyModifiers::empty(),
                        kind: KeyEventKind::Release,
                    }),
                    TerminalEvent::Key(KeyEvent {
                        code: KeyCode::Char('o'),
                        modifiers: KeyModifiers::empty(),
                        kind: KeyEventKind::Press,
                    }),
                    TerminalEvent::Key(KeyEvent {
                        code: KeyCode::Char('o'),
                        modifiers: KeyModifiers::empty(),
                        kind: KeyEventKind::Repeat,
                    }),
                    TerminalEvent::Key(KeyEvent {
                        code: KeyCode::Char('o'),
                        modifiers: KeyModifiers::empty(),
                        kind: KeyEventKind::Release,
                    }),
                ],
            )))
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .await;
        let expected = vec!["\n", "foo\n"];
        assert_eq!(actual, expected);
    }
}
