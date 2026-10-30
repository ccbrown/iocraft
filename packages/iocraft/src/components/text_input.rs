use crate::{
    component,
    components::{TextDecoration, TextDrawer, TextWrap, View},
    element,
    hooks::{Ref, State, UseMemo, UseState, UseTerminalEvents},
    segmented_string::{grapheme_width, SegmentedString, HARD_LINE_BREAKS},
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
    cursor: State<CursorPosition>,
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
            .map_or(0, |inner| inner.cursor.get().offset)
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
    /// If false, line breaks in pasted text are replaced with spaces.
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

// At a soft wrap, the end of one row and the start of the next share the same offset.
// `at_row_end` says which of the two the cursor is displayed at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CursorPosition {
    offset: usize,
    at_row_end: bool,
}

impl CursorPosition {
    fn new(offset: usize) -> Self {
        Self {
            offset,
            at_row_end: false,
        }
    }
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
        let rows = s
            .wrap(width)
            .into_iter()
            .map(|line| {
                let segment = &line.segments[0];
                TextBufferRow {
                    offset: segment.offset,
                    len: segment.text.len(),
                    width: line.width,
                }
            })
            .collect();
        Self { rows, text }
    }

    fn is_soft_wrapped(&self, row: usize) -> bool {
        let r = &self.rows[row];
        self.rows
            .get(row + 1)
            .is_some_and(|next| next.offset == r.offset + r.len)
    }

    // The row end position only exists at a soft wrap, which can move when the text is re-wrapped.
    fn normalize(&self, cursor: CursorPosition) -> CursorPosition {
        let at_soft_wrap = (0..self.rows.len()).any(|i| {
            let row = &self.rows[i];
            row.offset + row.len == cursor.offset && self.is_soft_wrapped(i)
        });
        if cursor.at_row_end && !at_soft_wrap {
            CursorPosition::new(cursor.offset)
        } else {
            cursor
        }
    }

    fn row_column_for_cursor(&self, cursor: CursorPosition) -> (u16, u16) {
        let offset = cursor.offset;
        for (i, row) in self.rows.iter().enumerate() {
            if offset >= row.offset
                && offset - row.offset <= row.len
                && (cursor.at_row_end || offset != row.offset + row.len || !self.is_soft_wrapped(i))
            {
                let col = grapheme_width(&self.text[row.offset..offset]) as u16;
                return (i as _, col);
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

    fn left_of(&self, cursor: CursorPosition) -> CursorPosition {
        CursorPosition::new(previous_grapheme_offset(&self.text, cursor.offset))
    }

    fn right_of(&self, cursor: CursorPosition) -> CursorPosition {
        CursorPosition::new(next_grapheme_offset(
            &self.text,
            cursor.offset.min(self.text.len()),
        ))
    }

    fn cursor_for_closest_column_in_row(&self, row: u16, col: u16) -> CursorPosition {
        let row_idx = row as usize;
        let row = &self.rows[row_idx];
        let col = col as usize;
        let mut width = 0;
        for (idx, grapheme) in self.text[row.offset..row.offset + row.len].grapheme_indices(true) {
            if width >= col {
                return CursorPosition::new(row.offset + idx);
            }
            width += grapheme.width();
        }
        CursorPosition {
            offset: row.offset + row.len,
            at_row_end: self.is_soft_wrapped(row_idx),
        }
    }

    fn above(&self, cursor: CursorPosition, col_preference: Option<u16>) -> CursorPosition {
        let (row, col) = self.row_column_for_cursor(cursor);
        if row == 0 {
            return cursor;
        }
        self.cursor_for_closest_column_in_row(row - 1, col_preference.unwrap_or(col))
    }

    fn below(&self, cursor: CursorPosition, col_preference: Option<u16>) -> CursorPosition {
        let (row, col) = self.row_column_for_cursor(cursor);
        if row as usize + 1 >= self.rows.len() {
            return cursor;
        }
        self.cursor_for_closest_column_in_row(row + 1, col_preference.unwrap_or(col))
    }

    fn row_start(&self, cursor: CursorPosition) -> CursorPosition {
        let (row, _) = self.row_column_for_cursor(cursor);
        CursorPosition::new(self.rows[row as usize].offset)
    }

    fn row_end(&self, cursor: CursorPosition) -> CursorPosition {
        let (row, _) = self.row_column_for_cursor(cursor);
        self.cursor_for_closest_column_in_row(row, u16::MAX)
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
    let mut cursor = hooks.use_state(CursorPosition::default);
    let mut requested_cursor_offset = hooks.use_state(|| None);
    let mut new_cursor_offset_hint = hooks.use_state(|| NewCursorOffsetHint::None);
    let mut scroll_offset_row = hooks.use_state(|| 0u16);
    let mut scroll_offset_col = hooks.use_state(|| 0u16);
    let mut vertical_movement_col_preference = hooks.use_state(|| None);
    let (width, height) = hooks.use_size();

    if let Some(handle_ref) = props.handle.as_mut() {
        handle_ref.set(TextInputHandle {
            inner: Some(TextInputHandleInner {
                cursor,
                requested_cursor_offset,
            }),
        });
    }

    let max_text_width = if wrap == TextWrap::Wrap {
        // Reserve the last column for the cursor.
        width.max(1) as usize - 1
    } else {
        usize::MAX
    };

    let buffer = hooks.use_memo(
        {
            let text = props.value.clone();
            move || Arc::new(TextBuffer::new(text, max_text_width))
        },
        (&props.value, max_text_width),
    );

    // Update the cursor position if the value has changed.
    if props.value.as_str() != prev_value.read().as_str() {
        let new_cursor = CursorPosition::new(new_cursor_offset(
            &prev_value.read(),
            cursor.get().offset,
            &props.value,
            new_cursor_offset_hint.get(),
        ));
        if cursor != new_cursor {
            cursor.set(new_cursor);
        }
        prev_value.set(props.value.clone());
        // XXX: Only reset the hint once the value changes. The parent may apply `on_change` in a
        // later render than the one triggered by the keypress.
        if new_cursor_offset_hint.get() != NewCursorOffsetHint::None {
            new_cursor_offset_hint.set(NewCursorOffsetHint::None);
        }
    }

    // Update the cursor position if the user requested it.
    if let Some(requested) = requested_cursor_offset.get() {
        let requested = CursorPosition::new(clamp_cursor_offset(&props.value, requested));
        if cursor != requested {
            cursor.set(requested);
        }
        requested_cursor_offset.set(None);
    }

    let normalized_cursor = buffer.normalize(cursor.get());
    if cursor != normalized_cursor {
        cursor.set(normalized_cursor);
    }

    let (cursor_row, mut cursor_col) = buffer.row_column_for_cursor(cursor.get());

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
        let content_width = grapheme_width(&props.value) as u16;
        let max_scroll_offset_col = (content_width + 1).saturating_sub(width);
        if scroll_offset_col.get() > max_scroll_offset_col {
            scroll_offset_col.set(max_scroll_offset_col);
        }
    }

    hooks.use_terminal_events({
        let buffer = buffer.clone();
        let mut value = props.value.clone();
        let mut temp_cursor_offset = cursor.get().offset;
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
                            cursor.set(buffer.row_start(cursor.get()));
                            vertical_movement_col_preference.set(None);
                        }
                        KeyCode::Char('e') => {
                            cursor.set(buffer.row_end(cursor.get()));
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
                                round_up_to_grapheme_boundary(&value, temp_cursor_offset);
                            new_cursor_offset_hint.set(NewCursorOffsetHint::Insertion);
                            on_change(value.clone());
                        }
                        KeyCode::Backspace => {
                            if temp_cursor_offset > 0 {
                                let end = temp_cursor_offset;
                                temp_cursor_offset = backspace_offset(&value, end);
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
                                    round_up_to_grapheme_boundary(&value, temp_cursor_offset);
                            }
                            on_change(value.clone());
                            new_cursor_offset_hint.set(NewCursorOffsetHint::Deletion);
                        }
                        KeyCode::Enter if multiline => {
                            value.insert(temp_cursor_offset, '\n');
                            temp_cursor_offset =
                                round_up_to_grapheme_boundary(&value, temp_cursor_offset + 1);
                            new_cursor_offset_hint.set(NewCursorOffsetHint::Insertion);
                            on_change(value.clone());
                        }
                        KeyCode::Left => {
                            cursor.set(buffer.left_of(cursor.get()));
                        }
                        KeyCode::Right => {
                            cursor.set(buffer.right_of(cursor.get()));
                        }
                        KeyCode::Up if multiline => {
                            clear_vertical_movement_col_preference = false;
                            if vertical_movement_col_preference.get().is_none() {
                                let (_, col) = buffer.row_column_for_cursor(cursor.get());
                                vertical_movement_col_preference.set(Some(col));
                            }
                            cursor.set(
                                buffer.above(cursor.get(), vertical_movement_col_preference.get()),
                            );
                        }
                        KeyCode::Down if multiline => {
                            clear_vertical_movement_col_preference = false;
                            if vertical_movement_col_preference.get().is_none() {
                                let (_, col) = buffer.row_column_for_cursor(cursor.get());
                                vertical_movement_col_preference.set(Some(col));
                            }
                            cursor.set(
                                buffer.below(cursor.get(), vertical_movement_col_preference.get()),
                            );
                        }
                        KeyCode::Home => {
                            cursor.set(buffer.row_start(cursor.get()));
                        }
                        KeyCode::End => {
                            cursor.set(buffer.row_end(cursor.get()));
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
                    temp_cursor_offset = round_up_to_grapheme_boundary(&value, temp_cursor_offset);
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
    if !data.contains(HARD_LINE_BREAKS) {
        return data;
    }
    let mut normalized = String::with_capacity(data.len());
    let mut chars = data.chars().peekable();
    while let Some(c) = chars.next() {
        if HARD_LINE_BREAKS.contains(&c) {
            if c == '\r' {
                chars.next_if_eq(&'\n');
            }
            normalized.push(' ');
        } else {
            normalized.push(c);
        }
    }
    normalized
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

// Backspace removes a single code point so that, for example, a combining mark can be corrected
// without retyping its base character. Emoji sequences and CRLF are removed whole, since their
// parts aren't meaningful on their own.
fn backspace_offset(value: &str, offset: usize) -> usize {
    let start = previous_grapheme_offset(value, offset);
    let grapheme = &value[start..offset];
    if grapheme == "\r\n" || grapheme.chars().any(is_emoji_sequence_component) {
        start
    } else {
        value[..offset]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }
}

// Every multi-code point emoji sequence contains at least one of these.
fn is_emoji_sequence_component(c: char) -> bool {
    matches!(
        c,
        '\u{200d}' // zero width joiner
            | '\u{fe0f}' // emoji presentation selector
            | '\u{20e3}' // combining enclosing keycap
            | '\u{1f1e6}'..='\u{1f1ff}' // regional indicators
            | '\u{1f3fb}'..='\u{1f3ff}' // skin tone modifiers
            | '\u{e0020}'..='\u{e007f}' // tags
    )
}

fn is_grapheme_boundary(value: &str, offset: usize) -> bool {
    GraphemeCursor::new(offset, value.len(), true)
        .is_boundary(value, 0)
        .unwrap()
}

fn round_up_to_grapheme_boundary(value: &str, offset: usize) -> usize {
    let mut offset = offset.min(value.len());
    while !value.is_char_boundary(offset) {
        offset += 1;
    }
    if is_grapheme_boundary(value, offset) {
        offset
    } else {
        next_grapheme_offset(value, offset)
    }
}

fn clamp_cursor_offset(value: &str, offset: usize) -> usize {
    let mut offset = offset.min(value.len());
    while !value.is_char_boundary(offset) {
        offset -= 1;
    }
    if is_grapheme_boundary(value, offset) {
        offset
    } else {
        previous_grapheme_offset(value, offset)
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

    // Offsets that land inside a grapheme are rounded forward after insertions and deletions, so
    // text before the cursor stays before it, and rounded back otherwise.
    let (offset, round_up) = if value.len() >= prev_value.len() && has_same_head && has_same_tail {
        // insertion (or no change)
        (
            cursor_offset + (value.len() - prev_value.len()),
            hint == NewCursorOffsetHint::Insertion,
        )
    } else if value.len() < prev_value.len() && has_same_tail && has_same_head {
        // ambiguous case, could be backspace or deletion
        if hint == NewCursorOffsetHint::Deletion {
            (cursor_offset, true)
        } else {
            // bias towards backspace
            (cursor_offset - (prev_value.len() - value.len()), false)
        }
    } else if value.len() < prev_value.len() && has_same_tail {
        // backspace
        (cursor_offset - (prev_value.len() - value.len()), false)
    } else if value.len() < prev_value.len() && has_same_head {
        // deletion
        (cursor_offset, true)
    } else {
        // unknown, put the cursor at the end
        (value.len(), false)
    };
    if round_up {
        round_up_to_grapheme_boundary(value, offset)
    } else {
        clamp_cursor_offset(value, offset)
    }
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
    struct TestInputProps {
        cursor: usize,
        #[iocraft(required)]
        width: u16,
        #[iocraft(required)]
        initial_value: &'static str,
        #[iocraft(required)]
        observed: Arc<Mutex<String>>,
    }

    #[component]
    fn TestInput(mut hooks: Hooks, props: &TestInputProps) -> impl Into<AnyElement<'static>> {
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

    #[derive(Props)]
    struct DeferredInputProps {
        #[iocraft(required)]
        initial_value: &'static str,
        #[iocraft(required)]
        observed: Arc<Mutex<String>>,
    }

    // Applies each change from `on_change` in a later render than the keypress.
    #[component]
    fn DeferredInput(
        mut hooks: Hooks,
        props: &DeferredInputProps,
    ) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut value = hooks.use_state(|| props.initial_value.to_string());
        let mut pending = hooks.use_state(|| None::<String>);
        let mut handle = hooks.use_ref_default::<TextInputHandle>();
        hooks.use_effect(move || handle.write().set_cursor_offset(1), ());
        hooks.use_future(async move {
            loop {
                smol::Timer::after(std::time::Duration::from_millis(50)).await;
                let next = pending.write().take();
                if let Some(next) = next {
                    value.set(next);
                }
            }
        });
        if value.read().contains('!') {
            system.exit();
        }
        let observed = props.observed.clone();
        element! {
            View(width: 11, height: 1) {
                TextInput(
                    has_focus: true,
                    handle,
                    value: value.to_string(),
                    on_change: move |new_value: String| {
                        *observed.lock().unwrap() = new_value.clone();
                        pending.set(Some(new_value));
                    },
                )
            }
        }
    }

    #[apply(test!)]
    async fn test_delete_hint_survives_deferred_value_update() {
        let observed = Arc::new(Mutex::new(String::new()));
        let mut app = element!(DeferredInput(
            initial_value: "aab",
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
                    KeyCode::Delete,
                )))
                .unwrap();
            // Wait for the deferred value to arrive.
            loop {
                let frame = frames.next().await.unwrap();
                if frame.get_text(0, 0, 11, 1).trim_end() == "ab" {
                    break;
                }
            }
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
            panic!("deferred input did not finish");
        })
        .await;
        // Delete keeps the cursor between the 'a's, so '!' lands there.
        assert_eq!(*observed.lock().unwrap(), "a!b");
    }

    #[apply(test!)]
    async fn test_soft_wrapped_row_end_editing() {
        for (initial_value, keys, expected) in [
            ("abcdef", &[KeyCode::End][..], "abc!def"),
            ("界界x", &[KeyCode::End], "界!界x"),
            ("abcdef", &[KeyCode::End, KeyCode::Home], "!abcdef"),
            ("abcdefghi", &[KeyCode::End, KeyCode::Down], "abcdef!ghi"),
            ("abcdef", &[KeyCode::End, KeyCode::Backspace], "ab!def"),
        ] {
            let observed = Arc::new(Mutex::new(String::new()));
            let mut app = element!(TestInput(
                cursor: 0_usize,
                width: 4_u16,
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
                for &code in keys.iter().chain([&KeyCode::Char('!')]) {
                    sender
                        .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                            KeyEventKind::Press,
                            code,
                        )))
                        .unwrap();
                    frames.next().await;
                }
                frames.for_each(|_| async {}).await;
            };
            smol::future::or(render, async {
                smol::Timer::after(std::time::Duration::from_secs(5)).await;
                panic!("wrapped input did not finish");
            })
            .await;
            assert_eq!(
                *observed.lock().unwrap(),
                expected,
                "{initial_value:?} {keys:?}"
            );
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
            ("ae\u{301}b", 4, Some(KeyCode::Backspace), "ae!b", "ae!b"),
            ("a🇺🇸b", 9, Some(KeyCode::Backspace), "a!b", "a!b"),
            (
                "a1\u{fe0f}\u{20e3}b",
                8,
                Some(KeyCode::Backspace),
                "a!b",
                "a!b",
            ),
            ("ae\u{301}b", 1, Some(KeyCode::Delete), "a!b", "a!b"),
            ("a👩‍👩‍👧‍👦b", 26, Some(KeyCode::Backspace), "a!b", "a!b"),
            ("a🇺🇸b", 1, Some(KeyCode::Delete), "a!b", "a!b"),
            (
                "a\n\u{301}",
                1,
                Some(KeyCode::Delete),
                "a\u{301}!",
                "a\u{301}!",
            ),
        ] {
            let observed = Arc::new(Mutex::new(String::new()));
            let events = key
                .into_iter()
                .chain(std::iter::once(KeyCode::Char('!')))
                .map(|code| TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, code)));
            let mut app = element!(TestInput(
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
                let mut app = element!(TestInput(
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
            let mut app = element!(TestInput(
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
        assert_eq!(buffer.row_column_for_cursor(at(1)), (0, 1));
        assert_eq!(buffer.row_column_for_cursor(at(3)), (1, 0));
        assert_eq!(buffer.row_column_for_cursor(at(5)), (2, 0));
        assert_eq!(buffer.row_end(at(0)), at(1));
        assert_eq!(buffer.left_of(at(3)), at(1));
        assert_eq!(buffer.right_of(at(1)), at(3));
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
            ("é\u{2028}界\u{b}🙂\u{85}", " é 界 🙂 ! \n"),
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

    fn at(offset: usize) -> CursorPosition {
        CursorPosition::new(offset)
    }

    fn at_row_end(offset: usize) -> CursorPosition {
        CursorPosition {
            offset,
            at_row_end: true,
        }
    }

    #[test]
    fn test_text_buffer_cursor_movement() {
        let buffer = TextBuffer::new("foo\nbar baz", 10);
        assert_eq!(buffer.left_of(at(2)), at(1));
        assert_eq!(buffer.right_of(at(2)), at(3));
        assert_eq!(buffer.above(at(2), None), at(2));
        assert_eq!(buffer.below(at(2), None), at(6));
        assert_eq!(buffer.below(at(2), Some(5)), at(9));
        assert_eq!(buffer.above(at(5), None), at(1));
        assert_eq!(buffer.below(at(5), None), at(5));
        assert_eq!(buffer.above(at(5), Some(6)), at(3));
    }

    #[test]
    fn test_grapheme_cursor_boundaries() {
        let buffer = TextBuffer::new("e\u{301}x\n👩‍👩‍👧‍👦x", 40);
        assert_eq!(buffer.right_of(at(0)), at(3));
        assert_eq!(buffer.left_of(at(3)), at(0));
        assert_eq!(clamp_cursor_offset(&buffer.text, 1), 0);
        assert_eq!(clamp_cursor_offset(&buffer.text, 2), 0);
        assert_eq!(buffer.below(at(3), None), at(30));
        assert_eq!(buffer.above(at(30), Some(1)), at(3));
    }

    #[test]
    fn test_soft_wrap_cursor_uses_next_row() {
        let buffer = TextBuffer::new("abcdefgh", 3);
        assert_eq!(buffer.row_column_for_cursor(at(3)), (1, 0));
        assert_eq!(buffer.row_start(at(3)), at(3));
        assert_eq!(buffer.above(at(3), None), at(0));
        assert_eq!(buffer.below(at(0), None), at(3));
    }

    #[test]
    fn test_soft_wrap_row_end() {
        for (text, end) in [("abcdef", 3), ("abc def", 4)] {
            let buffer = TextBuffer::new(text, 3);
            assert_eq!(buffer.row_end(at(1)), at_row_end(end), "{text:?}");
            assert_eq!(
                buffer.row_column_for_cursor(at_row_end(end)),
                (0, end as u16),
                "{text:?}"
            );
            assert_eq!(buffer.row_start(at_row_end(end)), at(0), "{text:?}");
            assert_eq!(buffer.row_end(at(end)), at(text.len()), "{text:?}");
        }
    }

    #[test]
    fn test_soft_wrap_row_end_moves_vertically() {
        let buffer = TextBuffer::new("abcdefghi", 3);
        assert_eq!(buffer.below(at_row_end(3), None), at_row_end(6));
        assert_eq!(buffer.above(at_row_end(6), None), at_row_end(3));
        assert_eq!(buffer.below(at_row_end(6), None), at(9));
        assert_eq!(buffer.above(at(9), None), at_row_end(6));
        assert_eq!(buffer.left_of(at_row_end(3)), at(2));
        assert_eq!(buffer.right_of(at_row_end(3)), at(4));
    }

    #[test]
    fn test_hard_line_end_does_not_use_row_end_position() {
        for separator in ["\n", "\u{2028}"] {
            let buffer = TextBuffer::new(format!("abc{separator}def"), 10);
            let next = 3 + separator.len();
            assert_eq!(buffer.row_end(at(0)), at(3), "{separator:?}");
            assert_eq!(buffer.above(at(next + 3), None), at(3), "{separator:?}");
            assert_eq!(
                buffer.row_column_for_cursor(at(next)),
                (1, 0),
                "{separator:?}"
            );
        }
    }

    #[test]
    fn test_row_end_position_is_dropped_when_no_longer_at_a_soft_wrap() {
        let narrow = TextBuffer::new("abcdef", 3);
        let wide = TextBuffer::new("abcdef", 10);
        assert_eq!(narrow.normalize(at_row_end(3)), at_row_end(3));
        assert_eq!(wide.normalize(at_row_end(3)), at(3));
        assert_eq!(narrow.normalize(at_row_end(6)), at(6));
    }

    #[apply(test!)]
    async fn test_empty_input_navigation() {
        for code in [
            KeyCode::Home,
            KeyCode::End,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Left,
            KeyCode::Right,
        ] {
            let observed = Arc::new(Mutex::new(String::new()));
            let mut app = element!(TestInput(
                width: 10_u16,
                initial_value: "",
                observed: observed.clone(),
            ));
            let events = [code, KeyCode::Char('!')]
                .into_iter()
                .map(|code| TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, code)));
            app.mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                events,
            )))
            .collect::<Vec<_>>()
            .await;
            assert_eq!(*observed.lock().unwrap(), "!", "{code:?}");
        }
    }

    #[test]
    fn test_up_stays_in_previous_wrapped_row() {
        let buffer = TextBuffer::new("abcdef", 3);
        assert_eq!(buffer.above(at(6), Some(3)), at_row_end(3));
    }

    #[test]
    fn test_down_does_not_skip_wrapped_row() {
        let buffer = TextBuffer::new("abcd\nabcdefgh", 4);
        assert_eq!(buffer.below(at(4), Some(4)), at_row_end(9));
        assert_eq!(buffer.below(at_row_end(9), Some(4)), at(13));
    }

    #[test]
    fn test_up_into_wide_grapheme_stays_in_previous_row() {
        let buffer = TextBuffer::new("👩‍👩x", 1);
        let (row, _) = buffer.row_column_for_cursor(buffer.above(at(buffer.text.len()), None));
        assert_eq!(row, 0);
    }

    #[test]
    fn test_test_buffer_row_column_for_offset() {
        assert_eq!(
            TextBuffer::new("一二!", 10).row_column_for_cursor(at(7)),
            (0, 5)
        );

        assert_eq!(
            TextBuffer::new("foo bar bazqux", 10).row_column_for_cursor(at(9)),
            (1, 1)
        );

        assert_eq!(
            TextBuffer::new("1234512345", 10).row_column_for_cursor(at(10)),
            (0, 10)
        );

        assert_eq!(
            TextBuffer::new("12345123451", 10).row_column_for_cursor(at(11)),
            (1, 1)
        );

        assert_eq!(
            TextBuffer::new("asd asd asd", 10).row_column_for_cursor(at(11)),
            (1, 3)
        );

        assert_eq!(
            TextBuffer::new("12345123 5 ", 10).row_column_for_cursor(at(11)),
            (0, 11)
        );

        assert_eq!(
            TextBuffer::new("asd\n", 10).row_column_for_cursor(at(4)),
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
