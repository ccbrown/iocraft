use crate::{
    component,
    components::{TextDecoration, TextDrawer, TextWrap, View},
    element,
    hooks::{Ref, State, UseMemo, UseState, UseTerminalEvents},
    segmented_string::SegmentedString,
    AnyElement, CanvasTextStyle, Color, Component, ComponentDrawer, ComponentUpdater, HandlerMut,
    Hook, Hooks, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, LayoutStyle, Overflow, Position,
    Props, Size, TerminalEvent, Weight,
};
use std::sync::Arc;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;

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
    /// Offsets past the end are clamped to the value's length. An offset
    /// inside a Unicode grapheme cluster is rounded down to that cluster's start.
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

    /// The weight (boldness) of the text.
    pub weight: Weight,

    /// The text decoration.
    pub decoration: TextDecoration,

    /// Whether the text is italicized.
    pub italic: bool,

    /// Whether the foreground and background colors are inverted.
    pub invert: bool,

    /// The current value.
    pub value: String,

    /// True if the input has focus and should process keyboard input.
    pub has_focus: bool,

    /// The handler to invoke when the value changes.
    pub on_change: HandlerMut<'static, String>,

    /// If true, the input will fill 100% of the height of its container and handle multiline input.
    /// When false, bracketed paste replaces LF, CR, and CRLF line breaks with spaces.
    /// Multiline paste preserves the original text.
    /// Cursor movement and deletion preserve Unicode grapheme clusters, including CRLF.
    /// After insertion, the cursor advances past any newly joined grapheme.
    /// At a soft wrap, the boundary offset belongs to the start of the following row.
    pub multiline: bool,

    /// If true (requires `multiline`), the input auto-grows its height to
    /// fit wrapped content instead of filling the container.
    pub auto_grow: bool,

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
    fn row_count(&self) -> usize {
        self.rows.len()
    }

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
                if offset_in_row <= row.len
                    && self
                        .rows
                        .get(i + 1)
                        .is_none_or(|next| next.offset != offset)
                {
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
        previous_grapheme_offset(&self.text, offset)
    }

    fn right_of_offset(&self, offset: usize) -> usize {
        next_grapheme_offset(&self.text, offset.min(self.text.len()))
    }

    fn offset_for_closest_column_in_row(&self, row: u16, col: u16) -> usize {
        let row_idx = row as usize;
        let row = &self.rows[row_idx];
        let col = col as usize;
        if col < row.width {
            let mut width = 0;
            for (idx, grapheme) in
                self.text[row.offset..row.offset + row.len].grapheme_indices(true)
            {
                if width >= col {
                    return row.offset + idx;
                }
                width += grapheme.width();
            }
        }
        let end = row.offset + row.len;
        if self
            .rows
            .get(row_idx + 1)
            .is_some_and(|next| next.offset == end)
        {
            previous_grapheme_offset(&self.text, end)
        } else {
            end
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

    fn row_start_offset(&self, offset: usize) -> usize {
        let (row, _) = self.row_column_for_offset(offset);
        self.rows[row as usize].offset
    }

    fn row_end_offset(&self, offset: usize) -> usize {
        let (row, _) = self.row_column_for_offset(offset);
        let r = &self.rows[row as usize];
        r.offset + r.len
    }
}

#[derive(Default, Props)]
struct TextBufferViewProps {
    auto_grow: bool,
    color: Option<Color>,
    weight: Weight,
    underline: bool,
    italic: bool,
    invert: bool,
    buffer: Arc<TextBuffer>,
}

#[derive(Default)]
struct TextBufferView {
    text_style: CanvasTextStyle,
    buffer: Arc<TextBuffer>,
    prev_row_count: usize,
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
            weight: props.weight,
            underline: props.underline,
            italic: props.italic,
            invert: props.invert,
        };
        self.buffer = props.buffer.clone();
        if props.auto_grow {
            // Auto-grow: use explicit height (row count) and full
            // parent width. Avoids set_measure_func entirely — no
            // mark_dirty, no render-loop interactions.
            let row_count = props.buffer.row_count().max(1);
            if row_count != self.prev_row_count {
                self.prev_row_count = row_count;
                updater.set_layout_style(
                    LayoutStyle {
                        width: Size::Percent(100.0),
                        height: Size::Length(row_count as u32),
                        ..Default::default()
                    }
                    .into(),
                );
            }
        } else {
            self.prev_row_count = 0;
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
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_>) {
        let mut drawer = TextDrawer::new(drawer, 0, false);
        drawer.append_lines(self.buffer.lines(), self.text_style, None);
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
    let auto_grow = props.auto_grow && multiline;
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
    }
    if new_cursor_offset_hint.get() != NewCursorOffsetHint::None {
        new_cursor_offset_hint.set(NewCursorOffsetHint::None);
    }

    // Update the cursor position if the user requested it.
    if let Some(requested) = requested_cursor_offset.get() {
        let requested = clamp_cursor_offset(&props.value, requested);
        if cursor_offset != requested {
            cursor_offset.set(requested);
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
        if auto_grow {
            if scroll_offset_col.get() != 0 {
                scroll_offset_col.set(0);
            }
        } else if cursor_col >= scroll_offset_col.get() + width {
            scroll_offset_col.set(cursor_col - width + 1);
        } else if cursor_col < scroll_offset_col.get() {
            scroll_offset_col.set(cursor_col as _);
        }

        // Scroll back when the content shrinks so it isn't cut off.
        let content_width = props.value.width() as u16;
        let max_scroll_offset_col = (content_width + 1).saturating_sub(width);
        if scroll_offset_col.get() > max_scroll_offset_col {
            scroll_offset_col.set(max_scroll_offset_col);
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
                    && modifiers.contains(KeyModifiers::CONTROL)
                    && !is_altgr_char(code, modifiers) =>
                {
                    match code {
                        KeyCode::Char('a') => {
                            cursor_offset.set(buffer.row_start_offset(cursor_offset.get()));
                            vertical_movement_col_preference.set(None);
                        }
                        KeyCode::Char('e') => {
                            cursor_offset.set(buffer.row_end_offset(cursor_offset.get()));
                            vertical_movement_col_preference.set(None);
                        }
                        _ => {}
                    }
                }
                TerminalEvent::Key(KeyEvent {
                    code,
                    kind,
                    modifiers,
                    ..
                }) if kind != KeyEventKind::Release
                    && (!modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                        || is_altgr_char(code, modifiers)) =>
                {
                    let mut clear_vertical_movement_col_preference = true;

                    match code {
                        KeyCode::Char(c) => {
                            value.insert(temp_cursor_offset, c);
                            temp_cursor_offset += c.len_utf8();
                            temp_cursor_offset =
                                cursor_offset_after_insertion(&value, temp_cursor_offset);
                            new_cursor_offset_hint.set(NewCursorOffsetHint::Insertion);
                            on_change(value.clone());
                        }
                        KeyCode::Backspace => {
                            if temp_cursor_offset > 0 {
                                let end = temp_cursor_offset;
                                temp_cursor_offset = previous_grapheme_offset(&value, end);
                                value.replace_range(temp_cursor_offset..end, "");
                                temp_cursor_offset =
                                    clamp_cursor_offset(&value, temp_cursor_offset);
                            }
                            on_change(value.clone());
                            new_cursor_offset_hint.set(NewCursorOffsetHint::Backspace);
                        }
                        KeyCode::Delete => {
                            if temp_cursor_offset < value.len() {
                                let end = next_grapheme_offset(&value, temp_cursor_offset);
                                value.replace_range(temp_cursor_offset..end, "");
                                temp_cursor_offset =
                                    clamp_cursor_offset(&value, temp_cursor_offset);
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
                        KeyCode::Home => {
                            cursor_offset.set(buffer.row_start_offset(cursor_offset.get()));
                        }
                        KeyCode::End => {
                            cursor_offset.set(buffer.row_end_offset(cursor_offset.get()));
                        }
                        _ => {
                            clear_vertical_movement_col_preference = false;
                        }
                    }

                    if clear_vertical_movement_col_preference {
                        vertical_movement_col_preference.set(None);
                    }
                }
                TerminalEvent::BracketedPaste(data) if !data.is_empty() => {
                    let data = if multiline {
                        data
                    } else {
                        normalize_single_line_paste(data)
                    };
                    value.insert_str(temp_cursor_offset, &data);
                    temp_cursor_offset += data.len();
                    temp_cursor_offset = cursor_offset_after_insertion(&value, temp_cursor_offset);
                    new_cursor_offset_hint.set(NewCursorOffsetHint::Insertion);
                    on_change(value.clone());
                    vertical_movement_col_preference.set(None);
                }
                _ => {}
            }
        }
    });

    element! {
        View(overflow: Overflow::Hidden, width: 100pct, height: if auto_grow { Size::Auto } else if multiline { Size::Percent(100.0) } else { Size::Length(1) }, position: Position::Relative) {
            View(position: if auto_grow { Position::Relative } else { Position::Absolute }, top: if auto_grow { 0 } else { -(scroll_offset_row.get() as i32) }, left: if auto_grow { 0 } else { -(scroll_offset_col.get() as i32) }) {
                #(if has_focus {
                    Some(element! {
                        View(position: Position::Absolute, top: cursor_row, left: cursor_col, width: 1, height: 1, background_color: props.cursor_color.unwrap_or(Color::Grey))
                    })
                } else {
                    None
                })
                TextBufferView(
                    auto_grow,
                    buffer,
                    color: props.color,
                    weight: props.weight,
                    underline: props.decoration == TextDecoration::Underline,
                    italic: props.italic,
                    invert: props.invert,
                )
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum NewCursorOffsetHint {
    #[default]
    None,
    Insertion,
    Backspace,
    Deletion,
}

fn normalize_single_line_paste(data: String) -> String {
    let Some(first_break) = data
        .as_bytes()
        .iter()
        .position(|byte| matches!(*byte, b'\r' | b'\n'))
    else {
        return data;
    };

    let mut bytes = data.into_bytes();
    let mut read = first_break;
    let mut write = first_break;
    while read < bytes.len() {
        let byte = bytes[read];
        bytes[write] = if matches!(byte, b'\r' | b'\n') {
            b' '
        } else {
            byte
        };
        read += 1;
        if byte == b'\r' && bytes.get(read) == Some(&b'\n') {
            read += 1;
        }
        write += 1;
    }
    bytes.truncate(write);
    // ASCII replacements/removals cannot invalidate the original UTF-8.
    String::from_utf8(bytes).expect("line break normalization preserves UTF-8")
}

/// Returns whether a keypress is using AltGr, always false for platforms other than Windows.
/// Windows reports AltGr (used for characters like '@', '{' and '€' on many keyboard layouts)
/// as Ctrl+Alt, so these key presses should be typed rather than treated as shortcuts.
/// Letters and digits are excluded, since they are used for shortcuts.
fn is_altgr_char(code: KeyCode, modifiers: KeyModifiers) -> bool {
    cfg!(windows)
        && modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::ALT)
        && matches!(code, KeyCode::Char(c) if !c.is_control() && !c.is_ascii_alphanumeric())
}

fn previous_grapheme_offset(value: &str, offset: usize) -> usize {
    GraphemeCursor::new(offset, value.len(), true)
        .prev_boundary(value, 0)
        .unwrap()
        .unwrap_or(0)
}

fn next_grapheme_offset(value: &str, offset: usize) -> usize {
    GraphemeCursor::new(offset, value.len(), true)
        .next_boundary(value, 0)
        .unwrap()
        .unwrap_or(value.len())
}

fn cursor_offset_after_insertion(value: &str, offset: usize) -> usize {
    let mut cursor = GraphemeCursor::new(offset, value.len(), true);
    if cursor.is_boundary(value, 0).unwrap() {
        offset
    } else {
        cursor
            .next_boundary(value, 0)
            .unwrap()
            .unwrap_or(value.len())
    }
}

fn clamp_cursor_offset(value: &str, offset: usize) -> usize {
    let mut offset = offset.min(value.len());
    while !value.is_char_boundary(offset) {
        offset -= 1;
    }
    let mut cursor = GraphemeCursor::new(offset, value.len(), true);
    if cursor.is_boundary(value, 0).unwrap() {
        offset
    } else {
        cursor.prev_boundary(value, 0).unwrap().unwrap_or(0)
    }
}

fn new_cursor_offset(
    prev_value: &str,
    cursor_offset: usize,
    value: &str,
    hint: NewCursorOffsetHint,
) -> usize {
    let has_same_head = value.len() >= cursor_offset
        && value.as_bytes()[..cursor_offset] == prev_value.as_bytes()[..cursor_offset];

    let tail_len = prev_value.len() - cursor_offset;
    let has_same_tail = value.len() >= tail_len
        && value.as_bytes()[value.len() - tail_len..] == prev_value.as_bytes()[cursor_offset..];

    let offset = if value.len() >= prev_value.len() && has_same_head && has_same_tail {
        // insertion (or no change)
        let offset = cursor_offset + (value.len() - prev_value.len());
        if hint == NewCursorOffsetHint::Insertion {
            return cursor_offset_after_insertion(value, offset);
        }
        offset
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
    };
    clamp_cursor_offset(value, offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use futures::stream::StreamExt;
    use macro_rules_attribute::apply;
    use smol_macros::test;
    use std::sync::{Arc, Mutex};

    #[derive(Props)]
    struct CrlfInputProps {
        cursor: usize,
        #[iocraft(required)]
        width: u16,
        #[iocraft(required)]
        initial_value: &'static str,
        #[iocraft(required)]
        observed: Arc<Mutex<String>>,
    }

    #[component]
    fn CrlfInput(mut hooks: Hooks, props: &CrlfInputProps) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut value = hooks.use_state(|| props.initial_value.to_string());
        let mut handle = hooks.use_ref_default::<TextInputHandle>();
        let cursor = props.cursor;
        hooks.use_effect(move || handle.write().set_cursor_offset(cursor), ());
        if value.read().contains('!') {
            system.exit();
        }
        let observed = props.observed.clone();
        element! {
            View(width: Size::Length(props.width as _), height: 3) {
                TextInput(
                    has_focus: true,
                    multiline: true,
                    handle,
                    value: value.to_string(),
                    on_change: move |new_value: String| {
                        *observed.lock().unwrap() = new_value.clone();
                        value.set(new_value);
                    },
                )
            }
        }
    }

    #[derive(Props)]
    struct AutoGrowToggleProps {
        #[iocraft(required)]
        state: Arc<Mutex<Option<State<bool>>>>,
    }

    #[component]
    fn AutoGrowToggle(
        mut hooks: Hooks,
        props: &AutoGrowToggleProps,
    ) -> impl Into<AnyElement<'static>> {
        let auto_grow = hooks.use_state(|| true);
        *props.state.lock().unwrap() = Some(auto_grow);
        element! {
            View(width: 12, height: 5, flex_direction: FlexDirection::Column) {
                Text(content: format!("auto={}", auto_grow.get()))
                TextInput(value: "a\nb", multiline: true, auto_grow: auto_grow.get())
                Text(content: "footer")
            }
        }
    }

    #[apply(test!)]
    async fn test_crlf_editing_and_requested_cursor_offsets() {
        for (initial_value, cursor, key, expected_value, expected_frame) in [
            ("a\r\nb", 3_usize, Some(KeyCode::Backspace), "a!b", "a!b"),
            ("a\r\nb", 1, Some(KeyCode::Delete), "a!b", "a!b"),
            ("a\r\nb", 2, None, "a!\r\nb", "a!\nb"),
            ("a\rb", 2, Some(KeyCode::Enter), "a\r\n!b", "a\n!b"),
            ("a界b", 4, Some(KeyCode::Backspace), "a!b", "a!b"),
            ("a界b", 1, Some(KeyCode::Delete), "a!b", "a!b"),
            ("ae\u{301}b", 4, Some(KeyCode::Backspace), "a!b", "a!b"),
            ("ae\u{301}b", 1, Some(KeyCode::Delete), "a!b", "a!b"),
            ("a👩‍👩‍👧‍👦b", 26, Some(KeyCode::Backspace), "a!b", "a!b"),
            ("a🇺🇸b", 1, Some(KeyCode::Delete), "a!b", "a!b"),
        ] {
            let observed = Arc::new(Mutex::new(String::new()));
            let events = key
                .into_iter()
                .chain(std::iter::once(KeyCode::Char('!')))
                .map(|code| TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, code)));
            let mut app = element!(CrlfInput(
                cursor: cursor,
                width: 11_u16,
                initial_value: initial_value,
                observed: observed.clone(),
            ));
            let frames = app
                .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                    events,
                )))
                .collect::<Vec<_>>()
                .await;
            assert_eq!(*observed.lock().unwrap(), expected_value);
            assert_eq!(
                frames.last().unwrap().get_text(0, 0, 11, 3).trim_end(),
                expected_frame
            );
        }
    }

    #[apply(test!)]
    async fn test_inserted_joiner_keeps_caret_after_merged_grapheme() {
        for (initial_value, expected) in [("👩👩", "👩‍👩!"), ("👩👩z", "👩‍👩!z")]
        {
            for insert in [
                TerminalEvent::BracketedPaste("\u{200d}".to_string()),
                TerminalEvent::Key(KeyEvent::new(
                    KeyEventKind::Press,
                    KeyCode::Char('\u{200d}'),
                )),
            ] {
                let observed = Arc::new(Mutex::new(String::new()));
                let mut app = element!(CrlfInput(
                    cursor: "👩".len(),
                    width: 11_u16,
                    initial_value: initial_value,
                    observed: observed.clone(),
                ));
                let render = async {
                    let (sender, events) = futures::channel::mpsc::unbounded();
                    let frames =
                        app.mock_terminal_render_loop(MockTerminalConfig::with_events(events));
                    futures::pin_mut!(frames);
                    frames.next().await.unwrap();
                    let _ = futures::poll!(frames.next());
                    let _ = futures::poll!(frames.next());
                    sender.unbounded_send(insert).unwrap();
                    frames.next().await.unwrap();
                    sender
                        .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                            KeyEventKind::Press,
                            KeyCode::Char('!'),
                        )))
                        .unwrap();
                    frames.for_each(|_| async {}).await;
                };
                smol::future::or(render, async {
                    smol::Timer::after(std::time::Duration::from_secs(5)).await;
                    panic!("joined grapheme input did not finish");
                })
                .await;
                assert_eq!(*observed.lock().unwrap(), expected);
            }
        }
    }

    #[apply(test!)]
    async fn test_wrapped_vertical_backspace_removes_whole_grapheme() {
        for (initial_value, expected) in [("👩‍👩x", "!x"), ("a \u{301}b", "! \u{301}b")] {
            let observed = Arc::new(Mutex::new(String::new()));
            let mut app = element!(CrlfInput(
                cursor: 0_usize,
                width: 2_u16,
                initial_value: initial_value,
                observed: observed.clone(),
            ));
            let render = async {
                let (sender, events) = futures::channel::mpsc::unbounded();
                let frames = app.mock_terminal_render_loop(MockTerminalConfig::with_events(events));
                futures::pin_mut!(frames);
                frames.next().await.unwrap();
                let _ = futures::poll!(frames.next());
                let _ = futures::poll!(frames.next());
                sender
                    .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                        KeyEventKind::Press,
                        KeyCode::Down,
                    )))
                    .unwrap();
                frames.next().await.unwrap();
                for code in [KeyCode::Backspace, KeyCode::Char('!')] {
                    sender
                        .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                            KeyEventKind::Press,
                            code,
                        )))
                        .unwrap();
                }
                frames.for_each(|_| async {}).await;
            };
            smol::future::or(render, async {
                smol::Timer::after(std::time::Duration::from_secs(5)).await;
                panic!("wrapped input did not finish");
            })
            .await;
            assert_eq!(*observed.lock().unwrap(), expected);
        }
    }

    #[test]
    fn test_crlf_rows_keep_raw_offsets_without_control_characters() {
        let buffer = TextBuffer::new("a\r\nb\r", usize::MAX);
        assert!(buffer.lines().eq(["a", "b", ""]));
        assert_eq!(buffer.row_column_for_offset(1), (0, 1));
        assert_eq!(buffer.row_column_for_offset(3), (1, 0));
        assert_eq!(buffer.row_column_for_offset(5), (2, 0));
        assert_eq!(buffer.row_end_offset(0), 1);
        assert_eq!(buffer.left_of_offset(3), 1);
        assert_eq!(buffer.right_of_offset(1), 3);
    }

    #[apply(test!)]
    async fn test_auto_grow_can_be_reenabled_without_changing_the_value() {
        let state = Arc::new(Mutex::new(None));
        let mut app = element!(AutoGrowToggle(state: state.clone()));
        let mut output = app.mock_terminal_render_loop(Default::default());
        for (step, expected_footer_row) in [3, 4, 3].into_iter().enumerate() {
            if step > 0 {
                let mut auto_grow = state.lock().unwrap().unwrap();
                auto_grow.set(step == 2);
            }
            let frame = output.next().await.unwrap().to_string();
            assert_eq!(
                frame.lines().position(|line| line == "footer"),
                Some(expected_footer_row),
                "{frame:?}"
            );
        }
    }

    #[derive(Default, Props)]
    struct MyComponentProps {
        initial_value: String,
        requested_cursor_offset: Option<usize>,
    }

    #[component]
    fn MyComponent(mut hooks: Hooks, props: &MyComponentProps) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut value = hooks.use_state(|| props.initial_value.clone());
        let mut handle = hooks.use_ref_default::<TextInputHandle>();
        let requested_cursor_offset = props.requested_cursor_offset;
        hooks.use_effect(
            move || {
                if let Some(offset) = requested_cursor_offset {
                    handle.write().set_cursor_offset(offset);
                }
            },
            (),
        );

        if value.read().contains("!") {
            system.exit();
        }

        element! {
            View(height: 1, width: 11, padding_left: 1) {
                TextInput(
                    has_focus: true,
                    value: value.to_string(),
                    on_change: move |new_value| value.set(new_value),
                    handle,
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
    async fn test_text_input_paste() {
        let actual = element!(MyComponent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                vec![TerminalEvent::BracketedPaste("hello!".to_string())],
            )))
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .await;
        let expected = vec!["  \n", " hello! \n"];
        assert_eq!(actual, expected);
    }

    #[apply(test!)]
    async fn test_text_input_paste_single_line_utf8_cursor() {
        for (pasted, expected) in [
            ("é\n界", " é 界! \n"),
            ("é\r界", " é 界! \n"),
            ("é\r\n界", " é 界! \n"),
            ("é\r\n界\r🙂\n", " é 界 🙂 ! \n"),
        ] {
            let actual = element!(MyComponent)
                .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                    vec![
                        TerminalEvent::BracketedPaste(pasted.to_string()),
                        TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('!'))),
                    ],
                )))
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .await;

            assert_eq!(actual, vec!["  \n", expected], "{pasted:?}");
        }
    }

    #[apply(test!)]
    async fn test_text_input_requested_cursor_boundaries() {
        for (offset, expected) in [(1, "!é界"), (2, "é!界"), (usize::MAX, "é界!")] {
            let frames = element! {
                MyComponent(initial_value: "é界", requested_cursor_offset: Some(offset))
            }
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                vec![TerminalEvent::Key(KeyEvent::new(
                    KeyEventKind::Press,
                    KeyCode::Char('!'),
                ))],
            )))
            .map(|canvas| canvas.to_string())
            .collect::<Vec<_>>()
            .await;

            assert_eq!(frames.last().unwrap().trim(), expected, "{offset}");
        }
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

    #[apply(test!)]
    async fn test_text_input_paste_multiline() {
        let actual = element!(MyMultilineComponent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                vec![TerminalEvent::BracketedPaste("red\nblue!".to_string())],
            )))
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .await;

        assert_eq!(actual, vec!["  \n\n\n", " red\n blue! \n\n"]);
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
    fn test_grapheme_cursor_boundaries() {
        let buffer = TextBuffer::new("e\u{301}x\n👩‍👩‍👧‍👦x", 40);
        assert_eq!(buffer.right_of_offset(0), 3);
        assert_eq!(buffer.left_of_offset(3), 0);
        assert_eq!(clamp_cursor_offset(&buffer.text, 1), 0);
        assert_eq!(clamp_cursor_offset(&buffer.text, 2), 0);
        assert_eq!(buffer.below_offset(3, None), 30);
        assert_eq!(buffer.above_offset(30, Some(1)), 3);
    }

    #[test]
    fn test_soft_wrap_cursor_uses_next_row() {
        let buffer = TextBuffer::new("abcdefgh", 3);
        assert_eq!(buffer.row_column_for_offset(3), (1, 0));
        assert_eq!(buffer.row_start_offset(3), 3);
        assert_eq!(buffer.row_end_offset(3), 6);
        assert_eq!(buffer.above_offset(3, None), 0);
        assert_eq!(buffer.below_offset(0, None), 3);
    }

    #[test]
    fn test_up_stays_in_previous_wrapped_row() {
        let buffer = TextBuffer::new("abcdef", 3);
        assert_eq!(buffer.above_offset(6, Some(3)), 2);
    }

    #[test]
    fn test_down_does_not_skip_wrapped_row() {
        let buffer = TextBuffer::new("abcd\nabcdefgh", 4);
        assert_eq!(buffer.below_offset(4, Some(4)), 8);
        assert_eq!(buffer.below_offset(8, Some(4)), 13);
    }

    #[test]
    fn test_up_into_wide_grapheme_stays_in_previous_row() {
        let buffer = TextBuffer::new("👩‍👩x", 1);
        assert_eq!(buffer.above_offset(buffer.text.len(), None), 0);
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
            new_cursor_offset("a\nb", 1, "a\r\nb", NewCursorOffsetHint::None),
            1
        );
        let mixed_text = "你好世界，Hello World";
        let cursor_offset = mixed_text.find('W').unwrap();
        assert_eq!(
            new_cursor_offset(
                mixed_text,
                cursor_offset,
                "你好世界，Hello aWorld",
                NewCursorOffsetHint::None
            ),
            cursor_offset + 1
        );

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

    #[cfg(windows)]
    #[test]
    fn test_is_altgr_char() {
        let ctrl_alt = KeyModifiers::CONTROL | KeyModifiers::ALT;
        assert!(is_altgr_char(KeyCode::Char('@'), ctrl_alt));
        assert!(is_altgr_char(KeyCode::Char('€'), ctrl_alt));
        assert!(!is_altgr_char(KeyCode::Char('a'), ctrl_alt));
        assert!(!is_altgr_char(KeyCode::Char('1'), ctrl_alt));
        assert!(!is_altgr_char(KeyCode::Char('@'), KeyModifiers::CONTROL));
    }

    #[cfg(not(windows))]
    #[test]
    fn test_is_altgr_char() {
        let ctrl_alt = KeyModifiers::CONTROL | KeyModifiers::ALT;
        assert!(!is_altgr_char(KeyCode::Char('@'), ctrl_alt));
    }
}
