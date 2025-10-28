use crate::{
    component,
    components::{TextDrawer, TextWrap, View},
    element,
    hooks::{Ref, State, UseMemo, UseState, UseTerminalEvents},
    segmented_string::SegmentedString,
    AnyElement, CanvasTextStyle, Color, Component, ComponentDrawer, ComponentUpdater, HandlerMut,
    Hook, Hooks, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, LayoutStyle, Overflow, Position,
    Props, Size, TerminalEvent,
};
use std::sync::Arc;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// A handle which can be used for imperative control of a [`TextInput`] component.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # #[component]
/// # fn FormField(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
/// # let mut value = hooks.use_state(|| "".to_string());
/// # let initial_cursor_position = 0;
/// let mut handle = hooks.use_ref_default::<TextInputHandle>();
///
/// hooks.use_effect(
///     move || handle.write().set_cursor_offset(initial_cursor_position),
///     (),
/// );
///
/// element! {
///     View(
///         background_color: Color::DarkGrey,
///         width: 30,
///     ) {
///         TextInput(
///             has_focus: true,
///             value: value.to_string(),
///             on_change: move |new_value| value.set(new_value),
///             handle,
///         )
///     }
/// }
/// # }
/// ```
#[derive(Default)]
pub struct TextInputHandle {
    inner: Option<TextInputHandleInner>,
}

struct TextInputHandleInner {
    cursor_offset: State<usize>,
    requested_cursor_offset: State<Option<usize>>,
}

impl TextInputHandle {
    /// Sets the cursor position to the specified offset.
    ///
    /// The offset is in bytes, not characters.
    pub fn set_cursor_offset(&mut self, offset: usize) {
        if let Some(inner) = &mut self.inner {
            inner.requested_cursor_offset.set(Some(offset));
        }
    }

    /// Gets the current cursor offset.
    ///
    /// The offset is in bytes, not characters.
    pub fn cursor_offset(&self) -> usize {
        self.inner
            .as_ref()
            .map_or(0, |inner| inner.cursor_offset.get())
    }
}

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
    pub on_change: HandlerMut<'static, String>,

    /// If true, the input will fill 100% of the height of its container and handle multiline input.
    pub multiline: bool,

    /// The color to make the cursor. Defaults to gray.
    pub cursor_color: Option<Color>,

    /// An optional handle which can be used for imperative control of the input.
    pub handle: Option<Ref<TextInputHandle>>,
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

struct TextBufferRow {
    offset: usize,
    len: usize,
    width: usize,
}

#[derive(Default)]
struct TextBuffer {
    text: String,
    rows: Vec<TextBufferRow>,
}

impl TextBuffer {
    fn new<S: Into<String>>(text: S, width: usize) -> Self {
        let text = text.into();
        let s = SegmentedString::from(text.as_str());
        let lines = s.wrap(width);
        let mut rows: Vec<TextBufferRow> = Vec::with_capacity(lines.len());
        for line in lines {
            rows.push(TextBufferRow {
                offset: line
                    .segments
                    .first()
                    .map(|s| s.offset)
                    .unwrap_or_else(|| rows.last().map_or(0, |r| r.offset)),
                len: line.segments.first().map_or(0, |s| s.text.len()),
                width: line.width,
            });
        }
        Self { rows, text }
    }

    fn row_column_for_offset(&self, offset: usize) -> (u16, u16) {
        for (i, row) in self.rows.iter().enumerate() {
            if offset >= row.offset {
                let offset_in_row = offset - row.offset;
                if offset_in_row <= row.len {
                    let col = self.text[row.offset..offset].width() as u16;
                    return (i as _, col);
                }
            }
        }
        (
            self.rows.len() as _,
            self.rows.last().map_or(0, |r| r.width as _),
        )
    }

    fn lines(&self) -> impl Iterator<Item = &str> {
        self.rows.iter().map(move |row| {
            let start = row.offset;
            let end = start + row.len;
            &self.text[start..end]
        })
    }

    fn left_of_offset(&self, offset: usize) -> usize {
        if offset == 0 {
            0
        } else {
            self.text[..offset]
                .char_indices()
                .last()
                .map_or(0, |(i, _)| i)
        }
    }

    fn right_of_offset(&self, offset: usize) -> usize {
        if offset >= self.text.len() {
            self.text.len()
        } else {
            self.text[offset..]
                .char_indices()
                .nth(1)
                .map_or(self.text.len(), |(i, _)| offset + i)
        }
    }

    fn offset_for_closest_column_in_row(&self, row: u16, col: u16) -> usize {
        let row = &self.rows[row as usize];
        let col = col as usize;
        if col >= row.width {
            row.offset + row.len
        } else {
            let mut width = 0;
            for (idx, c) in self.text[row.offset..].char_indices() {
                if width >= col {
                    return row.offset + idx;
                }
                width += c.width().unwrap_or(0);
            }
            row.offset + row.len
        }
    }

    fn above_offset(&self, offset: usize, col_preference: Option<u16>) -> usize {
        let (row, col) = self.row_column_for_offset(offset);
        if row == 0 {
            return offset;
        }
        self.offset_for_closest_column_in_row(row - 1, col_preference.unwrap_or(col))
    }

    fn below_offset(&self, offset: usize, col_preference: Option<u16>) -> usize {
        let (row, col) = self.row_column_for_offset(offset);
        if row as usize + 1 >= self.rows.len() {
            return offset;
        }
        self.offset_for_closest_column_in_row(row + 1, col_preference.unwrap_or(col))
    }
}

#[derive(Default, Props)]
struct TextBufferViewProps {
    color: Option<Color>,
    buffer: Arc<TextBuffer>,
}

#[derive(Default)]
struct TextBufferView {
    text_style: CanvasTextStyle,
    buffer: Arc<TextBuffer>,
}

impl Component for TextBufferView {
    type Props<'a> = TextBufferViewProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self::default()
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        self.text_style = CanvasTextStyle {
            color: props.color,
            ..Default::default()
        };
        self.buffer = props.buffer.clone();
        updater.set_layout_style(
            LayoutStyle {
                position: Position::Absolute,
                top: 0.into(),
                left: 0.into(),
                ..Default::default()
            }
            .into(),
        );
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_>) {
        let mut drawer = TextDrawer::new(drawer, false);
        drawer.append_lines(self.buffer.lines(), self.text_style);
    }
}

/// `TextInput` is a component that can receive text input from the user.
///
/// It will fill the available width and display the current value. Typically, you will want to
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
    let multiline = props.multiline;
    let has_focus = props.has_focus;
    let wrap = if multiline {
        TextWrap::Wrap
    } else {
        TextWrap::NoWrap
    };

    let mut prev_value = hooks.use_state(|| "".to_string());
    let mut cursor_offset = hooks.use_state(|| 0usize);
    let mut requested_cursor_offset = hooks.use_state(|| None);
    let mut new_cursor_offset_hint = hooks.use_state(|| NewCursorOffsetHint::None);
    let mut scroll_offset_row = hooks.use_state(|| 0u16);
    let mut scroll_offset_col = hooks.use_state(|| 0u16);
    let mut vertical_movement_col_preference = hooks.use_state(|| None);
    let (width, height) = hooks.use_size();

    if let Some(handle_ref) = props.handle.as_mut() {
        handle_ref.set(TextInputHandle {
            inner: Some(TextInputHandleInner {
                cursor_offset,
                requested_cursor_offset,
            }),
        });
    }

    let max_text_width = if wrap == TextWrap::Wrap {
        // Reserve the last column for the cursor.
        width.max(1) - 1
    } else {
        usize::MAX as _
    };

    let buffer = hooks.use_memo(
        {
            let text = props.value.clone();
            move || Arc::new(TextBuffer::new(text, max_text_width as _))
        },
        (&props.value, max_text_width),
    );

    // Update the cursor position if the value has changed.
    if props.value.as_str() != prev_value.read().as_str() {
        let new_cursor_offset = new_cursor_offset(
            &prev_value.read(),
            cursor_offset.get(),
            &props.value,
            new_cursor_offset_hint.get(),
        );
        if cursor_offset != new_cursor_offset {
            cursor_offset.set(new_cursor_offset);
        }
        prev_value.set(props.value.clone());
        new_cursor_offset_hint.set(NewCursorOffsetHint::None);
    }

    // Update the cursor position if the user requested it.
    if let Some(requested) = requested_cursor_offset.get() {
        if cursor_offset != requested {
            cursor_offset.set(requested.min(props.value.len()));
        }
        requested_cursor_offset.set(None);
    }

    let (cursor_row, mut cursor_col) = buffer.row_column_for_offset(cursor_offset.get());

    // If we're wrapping, don't let the cursor go past the visible area. No non-whitespace
    // characters will extend that far.
    if wrap == TextWrap::Wrap && cursor_col >= width && width > 0 {
        cursor_col = width - 1;
    }

    // Update the offset if the cursor is out of bounds.
    if width > 0 && height > 0 {
        if cursor_row >= scroll_offset_row.get() + height {
            scroll_offset_row.set(cursor_row - height + 1);
        } else if cursor_row < scroll_offset_row.get() {
            scroll_offset_row.set(cursor_row as _);
        }
        if cursor_col >= scroll_offset_col.get() + width {
            scroll_offset_col.set(cursor_col - width + 1);
        } else if cursor_col < scroll_offset_col.get() {
            scroll_offset_col.set(cursor_col as _);
        }
    }

    hooks.use_terminal_events({
        let buffer = buffer.clone();
        let mut value = props.value.clone();
        let mut temp_cursor_offset = cursor_offset.get();
        let mut on_change = props.on_change.take();
        move |event| {
            if !has_focus {
                return;
            }

            match event {
                TerminalEvent::Key(KeyEvent {
                    code,
                    kind,
                    modifiers,
                    ..
                }) if kind != KeyEventKind::Release
                    && !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    let mut clear_vertical_movement_col_preference = true;

                    match code {
                        KeyCode::Char(c) => {
                            value.insert(temp_cursor_offset, c);
                            temp_cursor_offset += c.len_utf8();
                            on_change(value.clone());
                        }
                        KeyCode::Backspace => {
                            if temp_cursor_offset > 0 {
                                temp_cursor_offset -= value[..temp_cursor_offset]
                                    .chars()
                                    .last()
                                    .unwrap()
                                    .len_utf8();
                                value.remove(temp_cursor_offset);
                            }
                            on_change(value.clone());
                            new_cursor_offset_hint.set(NewCursorOffsetHint::Backspace);
                        }
                        KeyCode::Delete => {
                            if temp_cursor_offset < value.len() {
                                value.remove(temp_cursor_offset);
                            }
                            on_change(value.clone());
                            new_cursor_offset_hint.set(NewCursorOffsetHint::Deletion);
                        }
                        KeyCode::Enter if multiline => {
                            value.insert(temp_cursor_offset, '\n');
                            temp_cursor_offset += 1;
                            on_change(value.clone());
                        }
                        KeyCode::Left => {
                            cursor_offset.set(buffer.left_of_offset(cursor_offset.get()));
                        }
                        KeyCode::Right => {
                            cursor_offset.set(buffer.right_of_offset(cursor_offset.get()));
                        }
                        KeyCode::Up if multiline => {
                            clear_vertical_movement_col_preference = false;
                            if vertical_movement_col_preference.get().is_none() {
                                let (_, col) = buffer.row_column_for_offset(cursor_offset.get());
                                vertical_movement_col_preference.set(Some(col));
                            }
                            cursor_offset.set(buffer.above_offset(
                                cursor_offset.get(),
                                vertical_movement_col_preference.get(),
                            ));
                        }
                        KeyCode::Down if multiline => {
                            clear_vertical_movement_col_preference = false;
                            if vertical_movement_col_preference.get().is_none() {
                                let (_, col) = buffer.row_column_for_offset(cursor_offset.get());
                                vertical_movement_col_preference.set(Some(col));
                            }
                            cursor_offset.set(buffer.below_offset(
                                cursor_offset.get(),
                                vertical_movement_col_preference.get(),
                            ));
                        }
                        _ => {
                            clear_vertical_movement_col_preference = false;
                        }
                    }

                    if clear_vertical_movement_col_preference {
                        vertical_movement_col_preference.set(None);
                    }
                }
                _ => {}
            }
        }
    });

    element! {
        View(overflow: Overflow::Hidden, width: 100pct, height: if multiline { Size::Percent(100.0) } else { Size::Length(1) }, position: Position::Relative) {
            View(position: Position::Absolute, top: -(scroll_offset_row.get() as i32), left: -(scroll_offset_col.get() as i32)) {
                #(if has_focus {
                    Some(element! {
                        View(position: Position::Absolute, top: cursor_row, left: cursor_col, width: 1, height: 1, background_color: props.cursor_color.unwrap_or(Color::Grey))
                    })
                } else {
                    None
                })
                TextBufferView(
                    buffer,
                    color: props.color,
                )
            }
        }
    }
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

    #[derive(Default, Props)]
    struct MyComponentProps {
        initial_value: String,
    }

    #[component]
    fn MyComponent(mut hooks: Hooks, props: &MyComponentProps) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut value = hooks.use_state(|| props.initial_value.clone());

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

    #[component]
    fn MyMultilineComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut value = hooks.use_state(|| "".to_string());

        if value.read().contains("!") {
            system.exit();
        }

        element! {
            View(height: 3, width: 11, padding_left: 1) {
                TextInput(
                    has_focus: true,
                    value: value.to_string(),
                    on_change: move |new_value| value.set(new_value),
                    multiline: true,
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
        let expected = vec!["  \n", " foo! \n"];
        assert_eq!(actual, expected);
    }

    #[apply(test!)]
    async fn test_text_input_initial_value() {
        let actual = element! {
            MyComponent(initial_value: "foo")
        }
        .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
            vec![
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('!'))),
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('!'))),
            ],
        )))
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .await;
        let expected = vec![" foo \n", " foo! \n"];
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
        let expected = vec!["  \n", " xxxxxxxx! \n"];
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
        let expected = vec!["  \n", " 一二! \n"];
        assert_eq!(actual, expected);
    }

    #[apply(test!)]
    async fn test_text_input_multiline_newline() {
        let actual = element!(MyMultilineComponent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                vec![
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('f'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('f'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('o'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('o'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('o'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('o'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('\n'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('\n'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('!'))),
                    TerminalEvent::Key(KeyEvent::new(KeyEventKind::Release, KeyCode::Char('!'))),
                ],
            )))
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .await;
        let expected = vec!["  \n\n\n", " foo\n ! \n\n"];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_text_buffer_cursor_movement() {
        let buffer = TextBuffer::new("foo\nbar baz", 10);
        assert_eq!(buffer.left_of_offset(2), 1);
        assert_eq!(buffer.right_of_offset(2), 3);
        assert_eq!(buffer.above_offset(2, None), 2);
        assert_eq!(buffer.below_offset(2, None), 6);
        assert_eq!(buffer.below_offset(2, Some(5)), 9);
        assert_eq!(buffer.above_offset(5, None), 1);
        assert_eq!(buffer.below_offset(5, None), 5);
        assert_eq!(buffer.above_offset(5, Some(6)), 3);
    }

    #[test]
    fn test_test_buffer_row_column_for_offset() {
        assert_eq!(
            TextBuffer::new("一二!", 10).row_column_for_offset(7),
            (0, 5)
        );

        assert_eq!(
            TextBuffer::new("foo bar bazqux", 10).row_column_for_offset(9),
            (1, 1)
        );

        assert_eq!(
            TextBuffer::new("1234512345", 10).row_column_for_offset(10),
            (0, 10)
        );

        assert_eq!(
            TextBuffer::new("12345123451", 10).row_column_for_offset(11),
            (1, 1)
        );

        assert_eq!(
            TextBuffer::new("asd asd asd", 10).row_column_for_offset(11),
            (1, 3)
        );

        assert_eq!(
            TextBuffer::new("12345123 5 ", 10).row_column_for_offset(11),
            (0, 11)
        );

        assert_eq!(
            TextBuffer::new("asd\n", 10).row_column_for_offset(4),
            (1, 0)
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
