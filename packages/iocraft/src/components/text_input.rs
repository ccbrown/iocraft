use crate::{
    component,
    components::{text::TEXT_DELIMITER, Text, TextWrap, View},
    element,
    hooks::{UseState, UseTerminalEvents},
    AnyElement, Color, ComponentDrawer, Handler, Hook, Hooks, KeyCode, KeyEvent, KeyEventKind,
    Overflow, Position, Props, Size, TerminalEvent,
};
use taffy::AvailableSpace;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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

    /// If true, the input will fill 100% of its container and handle multiline input.
    pub multiline: bool,
}

trait UseSize<'a> {
    fn use_size(&mut self) -> (u16, u16);
}

impl<'a> UseSize<'a> for Hooks<'a, '_> {
    fn use_size(&mut self) -> (u16, u16) {
        self.use_hook(UseSizeImpl::default).size
    }
}

#[derive(Default)]
struct UseSizeImpl {
    size: (u16, u16),
}

impl Hook for UseSizeImpl {
    fn pre_component_draw(&mut self, drawer: &mut ComponentDrawer) {
        let s = drawer.size();
        self.size = (s.width, s.height);
    }
}

/// `TextInput` is a component that can receive text input from the user.
///
/// It will fill the available space and display the current value. Typically, you will want to
/// render it in a [`View`](crate::components::View) component of the desired text field size.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # #[component]
/// # fn FormField(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
/// let mut value = hooks.use_state(|| "".to_string());
///
/// element! {
///     View(
///         border_style: BorderStyle::Round,
///         border_color: Color::Blue,
///     ) {
///         View(width: 15) {
///             Text(content: "Input: ")
///         }
///         View(
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
#[component]
pub fn TextInput(mut hooks: Hooks, props: &mut TextInputProps) -> impl Into<AnyElement<'static>> {
    let mut prev_value = hooks.use_state(|| "".to_string());
    let mut cursor_row = hooks.use_state(|| 0);
    let mut cursor_col = hooks.use_state(|| 0);
    let mut new_cursor_offset_hint = hooks.use_state(|| NewCursorOffsetHint::None);
    let mut offset_row = hooks.use_state(|| 0);
    let mut offset_col = hooks.use_state(|| 0);
    let size = hooks.use_size();
    let has_focus = props.has_focus;
    let multiline = props.multiline;
    let wrap = if multiline {
        TextWrap::Wrap
    } else {
        TextWrap::NoWrap
    };

    let mut cursor_offset = cursor_offset(&prev_value.read(), cursor_row.get(), cursor_col.get());

    // Update the cursor position if the value has changed.
    if props.value.as_str() != prev_value.read().as_str() {
        cursor_offset = new_cursor_offset(
            &prev_value.read(),
            cursor_offset,
            &props.value,
            new_cursor_offset_hint.get(),
        );
        let (row, col) = cursor_row_col(&props.value, cursor_offset);
        cursor_row.set(row);
        cursor_col.set(col);
        prev_value.set(props.value.clone());
        new_cursor_offset_hint.set(NewCursorOffsetHint::None);
    }

    let delimited_value = props.value[..cursor_offset].to_string()
        + TEXT_DELIMITER
        + &props.value[cursor_offset..]
        + TEXT_DELIMITER;

    // Due to wrapping and cursor movement across lines, the cursor position when rendered may not be the same.
    let text_width = size.0.max(1) - 1;
    let (display_cursor_row, display_cursor_col) = if wrap == TextWrap::NoWrap {
        (
            cursor_row.get(),
            cursor_col
                .get()
                .min(row_cols(&props.value, cursor_row.get())),
        )
    } else {
        display_cursor_row_col(&delimited_value, wrap, text_width)
    };

    // Update the offset if the displayed cursor is out of bounds.
    {
        if display_cursor_row >= offset_row.get() + size.1 {
            offset_row.set(display_cursor_row - size.1 + 1);
        } else if display_cursor_row < offset_row.get() {
            offset_row.set(display_cursor_row);
        }
        if display_cursor_col >= offset_col.get() + size.0 {
            offset_col.set(display_cursor_col - size.0 + 1);
        } else if display_cursor_col < offset_col.get() {
            offset_col.set(display_cursor_col);
        }
    }

    hooks.use_terminal_events({
        let mut value = props.value.clone();
        let mut on_change = props.on_change.take();
        move |event| {
            if !has_focus {
                return;
            }

            match event {
                TerminalEvent::Key(KeyEvent { code, kind, .. })
                    if kind != KeyEventKind::Release =>
                {
                    match code {
                        KeyCode::Char(c) => {
                            value.insert(cursor_offset, c);
                            cursor_offset += c.len_utf8();
                            on_change(value.clone());
                        }
                        KeyCode::Backspace => {
                            if cursor_offset > 0 {
                                cursor_offset -=
                                    value[..cursor_offset].chars().last().unwrap().len_utf8();
                                value.remove(cursor_offset);
                            }
                            on_change(value.clone());
                            new_cursor_offset_hint.set(NewCursorOffsetHint::Backspace);
                        }
                        KeyCode::Delete => {
                            if cursor_offset < value.len() {
                                value.remove(cursor_offset);
                            }
                            on_change(value.clone());
                            new_cursor_offset_hint.set(NewCursorOffsetHint::Deletion);
                        }
                        KeyCode::Enter => {
                            if multiline {
                                value.insert(cursor_offset, '\n');
                                cursor_offset += 1;
                                on_change(value.clone());
                            }
                        }
                        KeyCode::Left => {
                            if cursor_col.get() > 0 {
                                cursor_col.set(
                                    cursor_col.get().min(row_cols(&value, cursor_row.get())) - 1,
                                );
                            }
                        }
                        KeyCode::Right => {
                            let row = value
                                .lines()
                                .nth(cursor_row.get() as usize)
                                .unwrap_or_default();
                            if cursor_col.get() < row.width() as u16 {
                                cursor_col.set(cursor_col.get() + 1);
                            }
                        }
                        KeyCode::Up => {
                            if multiline && cursor_row.get() > 0 {
                                cursor_row.set(cursor_row.get() - 1);
                            }
                        }
                        KeyCode::Down => {
                            if multiline && cursor_row.get() < value.lines().count() as u16 - 1 {
                                cursor_row.set(cursor_row.get() + 1);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    });

    element! {
        View(overflow: Overflow::Hidden, width: 100pct, height: if multiline { Size::Percent(100.0) } else { Size::Length(1) }, position: Position::Relative) {
            View(position: Position::Absolute, top: -(offset_row.get() as i32), left: -(offset_col.get() as i32), width: 100pct) {
                #(if has_focus {
                    Some(element! {
                        View(position: Position::Absolute, top: display_cursor_row, left: display_cursor_col, width: 1, height: 1, background_color: Color::Grey)
                    })
                } else {
                    None
                })
                View(width: Size::Length(text_width as _)) {
                    Text(
                        content: &delimited_value,
                        color: props.color,
                        wrap,
                    )
                }
            }
        }
    }
}

fn display_cursor_row_col(delimited_value: &str, wrap: TextWrap, width: u16) -> (u16, u16) {
    let wrapped = Text::wrap(
        delimited_value,
        wrap,
        None,
        AvailableSpace::Definite(width as _),
    );
    for (i, line) in wrapped.lines().enumerate() {
        if let Some(offset) = line.find(TEXT_DELIMITER) {
            return (i as u16, line[..offset].width() as u16);
        }
    }
    unreachable!("there should always be a line containing the delimiter");
}

fn row_cols(value: &str, row: u16) -> u16 {
    let row = value.lines().nth(row as usize).unwrap_or_default();
    row.width() as u16
}

fn cursor_row_col(value: &str, offset: usize) -> (u16, u16) {
    let mut row = 0;
    let mut col = 0;
    let mut current_offset = 0;

    for c in value.chars() {
        if current_offset >= offset {
            break;
        }
        if c == '\n' {
            row += 1;
            col = 0;
        } else {
            col += c.width().unwrap_or(1) as u16;
        }
        current_offset += c.len_utf8();
    }

    (row, col)
}

fn cursor_offset(value: &str, row: u16, col: u16) -> usize {
    if row == 0 && col == 0 {
        return 0;
    }

    let mut offset = 0;
    let mut current_row = 0;
    let mut current_col = 0;

    for c in value.chars() {
        if c == '\n' {
            if current_row == row {
                break;
            }
            current_row += 1;
            current_col = 0;
        } else {
            current_col += c.width().unwrap_or(1) as u16;
        }

        offset += c.len_utf8();
        if current_row == row && current_col >= col {
            break;
        }
    }

    offset
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum NewCursorOffsetHint {
    #[default]
    None,
    Backspace,
    Deletion,
}

fn new_cursor_offset(
    prev_value: &str,
    cursor_offset: usize,
    value: &str,
    hint: NewCursorOffsetHint,
) -> usize {
    let has_same_head = value.len() >= cursor_offset
        && value
            .chars()
            .zip(prev_value.chars())
            .take(cursor_offset)
            .all(|(a, b)| a == b);

    let tail_len = prev_value.len() - cursor_offset;
    let has_same_tail = value.len() >= tail_len
        && value
            .chars()
            .rev()
            .zip(prev_value.chars().rev())
            .take(tail_len)
            .all(|(a, b)| a == b);

    if value.len() >= prev_value.len() && has_same_head && has_same_tail {
        // insertion (or no change)
        cursor_offset + (value.len() - prev_value.len())
    } else if value.len() < prev_value.len() && has_same_tail && has_same_head {
        // ambiguous case, could be backspace or deletion
        if hint == NewCursorOffsetHint::Deletion {
            cursor_offset
        } else {
            // bias towards backspace
            cursor_offset - (prev_value.len() - value.len())
        }
    } else if value.len() < prev_value.len() && has_same_tail {
        // backspace
        cursor_offset - (prev_value.len() - value.len())
    } else if value.len() < prev_value.len() && has_same_head {
        // deletion
        cursor_offset
    } else {
        // unknown, put the cursor at the end
        value.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use futures::stream::StreamExt;
    use macro_rules_attribute::apply;
    use smol_macros::test;

    #[component]
    fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut value = hooks.use_state(|| "".to_string());

        if value.read().contains("!") {
            system.exit();
        }

        element! {
            View(height: 1, width: 11, padding_left: 1) {
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
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('f'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('f'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('o'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Repeat, KeyCode::Char('o'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('o'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('!'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('!'))),
                ],
            )))
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .await;
        let expected = vec!["\n", "  \n", " foo! \n"];
        assert_eq!(actual, expected);
    }

    #[apply(test!)]
    async fn test_text_input_overflow() {
        let actual = element!(MyComponent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                vec![
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Repeat, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Repeat, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Repeat, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Repeat, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Repeat, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Repeat, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Repeat, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Repeat, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Repeat, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Repeat, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Repeat, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Repeat, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('x'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('!'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('!'))),
                ],
            )))
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .await;
        let expected = vec!["\n", "  \n", " xxxxxxxx! \n"];
        assert_eq!(actual, expected);
    }

    #[apply(test!)]
    async fn test_text_input_kanji() {
        let actual = element!(MyComponent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                vec![
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('一'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('一'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('二'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('二'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('!'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('!'))),
                ],
            )))
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .await;
        let expected = vec!["\n", "  \n", " 一二! \n"];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_cursor_offset() {
        assert_eq!(cursor_offset("foo", 0, 0), 0);
        assert_eq!(cursor_offset("foo", 0, 1), 1);
        assert_eq!(cursor_offset("foo", 0, 6), 3);
        assert_eq!(cursor_offset("foo\nbar", 0, 6), 3);
        assert_eq!(cursor_offset("日本", 0, 2), 3);
    }

    #[test]
    fn test_display_cursor_row_col() {
        assert_eq!(
            display_cursor_row_col(
                &format!("foo bar b{}azqux", TEXT_DELIMITER),
                TextWrap::Wrap,
                10
            ),
            (1, 1)
        );

        assert_eq!(
            display_cursor_row_col(&format!("1234512345{}", TEXT_DELIMITER), TextWrap::Wrap, 10),
            (0, 10)
        );

        assert_eq!(
            display_cursor_row_col(
                &format!("12345123451{}", TEXT_DELIMITER),
                TextWrap::Wrap,
                10
            ),
            (1, 1)
        );

        assert_eq!(
            display_cursor_row_col(
                &format!("asd asd asd{}", TEXT_DELIMITER),
                TextWrap::Wrap,
                10
            ),
            (1, 3)
        );

        assert_eq!(
            display_cursor_row_col(
                &format!("12345123 5 {}", TEXT_DELIMITER),
                TextWrap::Wrap,
                10
            ),
            (1, 2)
        );
    }

    #[test]
    fn test_new_cursor_offset() {
        assert_eq!(
            new_cursor_offset("", 0, "foo", NewCursorOffsetHint::None),
            3
        );
        assert_eq!(
            new_cursor_offset("foo", 3, "foobar", NewCursorOffsetHint::None),
            6
        );
        assert_eq!(
            new_cursor_offset("foobar", 3, "foobar", NewCursorOffsetHint::None),
            3
        );
        assert_eq!(
            new_cursor_offset("foobar", 3, "fooar", NewCursorOffsetHint::None),
            3
        );
        assert_eq!(
            new_cursor_offset("foobar", 3, "fooasdbar", NewCursorOffsetHint::None),
            6
        );
        assert_eq!(
            new_cursor_offset("a\n", 0, "\n", NewCursorOffsetHint::None),
            0
        );
        assert_eq!(
            new_cursor_offset(
                "asddasd\nasdasd",
                3,
                "asdasd\nasdasd",
                NewCursorOffsetHint::Backspace
            ),
            2
        );
        assert_eq!(
            new_cursor_offset(
                "asddasd\nasdasd",
                3,
                "asdasd\nasdasd",
                NewCursorOffsetHint::Deletion
            ),
            3
        );
    }
}
