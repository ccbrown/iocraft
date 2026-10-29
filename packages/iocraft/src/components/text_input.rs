use crate::{
    component,
    components::{TextDecoration, TextDrawer, TextWrap, View},
    element,
    hooks::{Ref, State, UseMemo, UseRef, UseState, UseTerminalEvents},
    segmented_string::SegmentedString,
    AnyElement, CanvasTextStyle, Color, Component, ComponentDrawer, ComponentUpdater, HandlerMut,
    Hook, Hooks, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, LayoutStyle, Overflow, Position,
    Props, Size, TerminalEvent, Weight,
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
    /// Offsets past the end are clamped to the value's length. An offset
    /// inside a UTF-8 character is rounded down to that character's start.
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
    /// The parent controls `value` and may normalize or reject each change.
    pub on_change: HandlerMut<'static, String>,

    /// If true, the input will fill 100% of the height of its container and handle multiline input.
    /// When false, bracketed paste replaces LF, CR, and CRLF line breaks with spaces.
    /// Multiline paste preserves the original text.
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
    fn use_size(&mut self) -> Ref<(u16, u16)>;
}

impl<'a> UseSize<'a> for Hooks<'a, '_> {
    fn use_size(&mut self) -> Ref<(u16, u16)> {
        let size = self.use_ref_default::<(u16, u16)>();
        self.use_hook(|| UseSizeImpl { size });
        size
    }
}

struct UseSizeImpl {
    size: Ref<(u16, u16)>,
}

impl Hook for UseSizeImpl {
    fn pre_component_draw(&mut self, drawer: &mut ComponentDrawer) {
        let s = drawer.size();
        self.size.set((s.width, s.height));
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

#[derive(Clone, Copy)]
struct CursorAnchor {
    position: usize,
    action_index: usize,
}

#[derive(Clone, Copy)]
enum TextEdit {
    Insert { offset: usize, len: usize },
    Remove { offset: usize, character: char },
}

impl TextEdit {
    fn undo(self, value: &mut String) {
        match self {
            Self::Insert { offset, len } => value.replace_range(offset..offset + len, ""),
            Self::Remove { offset, character } => value.insert(offset, character),
        }
    }
}

struct CursorCheckpoint {
    edit: TextEdit,
    anchor: CursorAnchor,
    unchanged_head: usize,
    unchanged_tail: usize,
}

impl CursorCheckpoint {
    fn changed_range(&self, initial: &str, value: &str) -> Option<(usize, usize)> {
        // The untouched edges do not need to be compared again.
        changed_byte_range(
            &initial[self.unchanged_head..initial.len() - self.unchanged_tail],
            &value[self.unchanged_head..value.len() - self.unchanged_tail],
        )
        .map(|(start, end)| (self.unchanged_head + start, self.unchanged_head + end))
    }
}

#[derive(Clone, Copy)]
enum CursorAction {
    Move(CursorMovement),
    ClearPreference,
}

#[derive(Default)]
struct TextInputEditState {
    last_prop_value: String,
    cursor_on_rejection: Option<(usize, Option<u16>)>,
    latest_edit: Option<CursorCheckpoint>,
    checkpoints: Vec<CursorCheckpoint>,
    actions: Vec<CursorAction>,
}

impl TextInputEditState {
    fn record_edit(
        &mut self,
        edit: TextEdit,
        before_position: usize,
        position: usize,
        value_len: usize,
    ) {
        let (head, tail) = if let Some(previous) = &self.latest_edit {
            (previous.unchanged_head, previous.unchanged_tail)
        } else {
            self.cursor_on_rejection = Some((before_position, None));
            (self.last_prop_value.len(), self.last_prop_value.len())
        };
        let (offset, end) = match edit {
            TextEdit::Insert { offset, len } => (offset, offset + len),
            TextEdit::Remove { offset, .. } => (offset, offset),
        };
        let checkpoint = CursorCheckpoint {
            edit,
            anchor: CursorAnchor {
                position,
                action_index: self.actions.len(),
            },
            unchanged_head: head.min(offset),
            unchanged_tail: tail.min(value_len - end),
        };
        if let Some(previous) = self.latest_edit.replace(checkpoint) {
            self.checkpoints.push(previous);
        }
    }

    fn record_movement(&mut self, movement: CursorMovement) {
        if self.latest_edit.is_some() {
            self.actions.push(CursorAction::Move(movement));
        }
    }

    fn clear_preference(&mut self) {
        if matches!(
            self.actions.last(),
            Some(CursorAction::Move(
                CursorMovement::Up | CursorMovement::Down
            ))
        ) {
            self.actions.push(CursorAction::ClearPreference);
        }
    }

    fn accepted_anchor(
        &self,
        pending_value: &str,
        accepted_value: &str,
        prop_changed: bool,
    ) -> Option<CursorAnchor> {
        if self.checkpoints.is_empty() {
            return None;
        }
        let latest = self.latest_edit.as_ref()?;
        let mut value = pending_value.to_owned();
        let accepted_range = if prop_changed {
            changed_byte_range(&self.last_prop_value, accepted_value)
        } else {
            None
        };
        let mut normalized = None;
        for checkpoint in std::iter::once(latest).chain(self.checkpoints.iter().rev()) {
            if value == accepted_value {
                return Some(checkpoint.anchor);
            }
            if normalized.is_none()
                && accepted_range.is_some_and(|range| {
                    checkpoint.changed_range(&self.last_prop_value, &value) == Some(range)
                })
            {
                normalized = Some(CursorAnchor {
                    position: new_cursor_offset(&value, checkpoint.anchor.position, accepted_value),
                    action_index: checkpoint.anchor.action_index,
                });
            }
            checkpoint.edit.undo(&mut value);
        }
        normalized
    }

    fn reconciled_cursor(
        &self,
        pending_value: &str,
        position: usize,
        preference: Option<u16>,
        buffer: &TextBuffer,
        prop_changed: bool,
    ) -> (usize, Option<u16>) {
        let (mut position, mut preference, action_index) =
            if let Some(anchor) = self.accepted_anchor(pending_value, &buffer.text, prop_changed) {
                (anchor.position, None, anchor.action_index)
            } else if let Some((position, preference)) =
                self.cursor_on_rejection.filter(|_| !prop_changed)
            {
                (position, preference, 0)
            } else if let Some(checkpoint) = &self.latest_edit {
                (
                    new_cursor_offset(pending_value, checkpoint.anchor.position, &buffer.text),
                    None,
                    checkpoint.anchor.action_index,
                )
            } else {
                (
                    if prop_changed {
                        new_cursor_offset(pending_value, position, &buffer.text)
                    } else {
                        position
                    },
                    preference,
                    self.actions.len(),
                )
            };
        for &action in &self.actions[action_index..] {
            match action {
                CursorAction::Move(movement) => {
                    position = cursor_after_movement(buffer, position, movement, &mut preference);
                    if !matches!(movement, CursorMovement::Up | CursorMovement::Down) {
                        preference = None;
                    }
                }
                CursorAction::ClearPreference => preference = None,
            }
        }
        (position, preference)
    }

    fn clear_pending(&mut self) {
        self.cursor_on_rejection = None;
        self.latest_edit = None;
        self.checkpoints.clear();
        self.actions.clear();
    }
}

/// `TextInput` is a component that can receive text input from the user.
///
/// It will fill the available width and display the current value. Typically, you will want to
/// render it in a [`View`](crate::components::View) component of the desired text field size.
/// Events received before the next redraw share the updated text and cursor position.
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

    // Keep pending edits separate from accepted props so rejected changes do not move the caret.
    let mut prev_value = hooks.use_state(|| "".to_string());
    let mut edit_state = hooks.use_ref(TextInputEditState::default);
    let mut cursor_offset = hooks.use_state(|| 0usize);
    let mut requested_cursor_offset = hooks.use_state(|| None);
    let mut scroll_offset_row = hooks.use_state(|| 0u16);
    let mut scroll_offset_col = hooks.use_state(|| 0u16);
    let mut vertical_movement_col_preference = hooks.use_state(|| None);
    let size = hooks.use_size();
    let (width, height) = *size.read();

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

    let (prop_changed, has_pending_edits) = {
        let edit = edit_state.read();
        (
            edit.last_prop_value != props.value,
            edit.latest_edit.is_some(),
        )
    };
    if props.value.as_str() != prev_value.read().as_str() {
        let (position, preference) = edit_state.read().reconciled_cursor(
            &prev_value.read(),
            cursor_offset.get(),
            vertical_movement_col_preference.get(),
            &buffer,
            prop_changed,
        );
        if vertical_movement_col_preference.get() != preference {
            vertical_movement_col_preference.set(preference);
        }
        if cursor_offset != position {
            cursor_offset.set(position);
        }
        prev_value.set(props.value.clone());
    }
    if prop_changed || has_pending_edits {
        let mut edit = edit_state.write();
        if prop_changed {
            edit.last_prop_value.clone_from(&props.value);
        }
        edit.clear_pending();
    }

    // Update the cursor position if the user requested it.
    if let Some(mut requested) = requested_cursor_offset.get() {
        requested = requested.min(props.value.len());
        while !props.value.is_char_boundary(requested) {
            requested -= 1;
        }
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
        let mut buffer = buffer.clone();
        let mut buffer_width = max_text_width;
        let mut buffer_dirty = false;
        let mut on_change = props.on_change.take();
        move |event| {
            if !has_focus {
                return;
            }

            let old_position = cursor_offset.get();
            let mut position = old_position;
            let mut changed_value = None;
            let mut movement = None;
            let mut clear_vertical_preference = false;
            let mut text_edit = None;

            let code = match event {
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
                        KeyCode::Char('a') => Some(KeyCode::Home),
                        KeyCode::Char('e') => Some(KeyCode::End),
                        _ => None,
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
                    Some(code)
                }
                TerminalEvent::BracketedPaste(data) if !data.is_empty() => {
                    let data = if multiline {
                        data
                    } else {
                        normalize_single_line_paste(data)
                    };
                    let mut value = prev_value.write();
                    text_edit = Some(TextEdit::Insert {
                        offset: position,
                        len: data.len(),
                    });
                    value.insert_str(position, &data);
                    position += data.len();
                    changed_value = Some(value.clone());
                    buffer_dirty = true;
                    clear_vertical_preference = true;
                    None
                }
                _ => None,
            };

            if let Some(code) = code {
                clear_vertical_preference = true;
                match code {
                    KeyCode::Char(c) => {
                        let mut value = prev_value.write();
                        text_edit = Some(TextEdit::Insert {
                            offset: position,
                            len: c.len_utf8(),
                        });
                        value.insert(position, c);
                        position += c.len_utf8();
                        changed_value = Some(value.clone());
                        buffer_dirty = true;
                    }
                    KeyCode::Backspace => {
                        let mut value = prev_value.write();
                        if position > 0 {
                            position -= value[..position].chars().last().unwrap().len_utf8();
                            let character = value.remove(position);
                            text_edit = Some(TextEdit::Remove {
                                offset: position,
                                character,
                            });
                            buffer_dirty = true;
                        }
                        changed_value = Some(value.clone());
                    }
                    KeyCode::Delete => {
                        let mut value = prev_value.write();
                        if position < value.len() {
                            let character = value.remove(position);
                            text_edit = Some(TextEdit::Remove {
                                offset: position,
                                character,
                            });
                            buffer_dirty = true;
                        }
                        changed_value = Some(value.clone());
                    }
                    KeyCode::Enter if multiline => {
                        let mut value = prev_value.write();
                        text_edit = Some(TextEdit::Insert {
                            offset: position,
                            len: 1,
                        });
                        value.insert(position, '\n');
                        position += 1;
                        changed_value = Some(value.clone());
                        buffer_dirty = true;
                    }
                    KeyCode::Left => movement = Some(CursorMovement::Left),
                    KeyCode::Right => movement = Some(CursorMovement::Right),
                    KeyCode::Home => movement = Some(CursorMovement::Home),
                    KeyCode::End => movement = Some(CursorMovement::End),
                    KeyCode::Up if multiline => {
                        movement = Some(CursorMovement::Up);
                        clear_vertical_preference = false;
                    }
                    KeyCode::Down if multiline => {
                        movement = Some(CursorMovement::Down);
                        clear_vertical_preference = false;
                    }
                    _ => clear_vertical_preference = false,
                }
            }

            if let Some(movement) = movement {
                let current_width = if multiline {
                    size.read().0.max(1) - 1
                } else {
                    max_text_width
                };
                if buffer_dirty || buffer_width != current_width {
                    buffer = Arc::new(TextBuffer::new(
                        prev_value.read().clone(),
                        current_width as _,
                    ));
                    buffer_width = current_width;
                    buffer_dirty = false;
                }
                let mut preference = vertical_movement_col_preference.get();
                position = cursor_after_movement(&buffer, position, movement, &mut preference);
                if vertical_movement_col_preference.get() != preference {
                    vertical_movement_col_preference.set(preference);
                }
                edit_state.write().record_movement(movement);
            }
            if clear_vertical_preference {
                vertical_movement_col_preference.set(None);
                edit_state.write().clear_preference();
            }
            if old_position != position {
                cursor_offset.set(position);
            }
            if let Some(value) = changed_value {
                if let Some(edit) = text_edit {
                    edit_state
                        .write()
                        .record_edit(edit, old_position, position, value.len());
                }
                on_change(value);
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

#[derive(Clone, Copy)]
enum CursorMovement {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
}

fn cursor_after_movement(
    buffer: &TextBuffer,
    position: usize,
    movement: CursorMovement,
    col_preference: &mut Option<u16>,
) -> usize {
    match movement {
        CursorMovement::Left => buffer.left_of_offset(position),
        CursorMovement::Right => buffer.right_of_offset(position),
        CursorMovement::Home | CursorMovement::End if buffer.text.is_empty() => 0,
        CursorMovement::Home => buffer.row_start_offset(position),
        CursorMovement::End => buffer.row_end_offset(position),
        CursorMovement::Up | CursorMovement::Down => {
            if col_preference.is_none() {
                *col_preference = Some(buffer.row_column_for_offset(position).1);
            }
            if matches!(movement, CursorMovement::Up) {
                buffer.above_offset(position, *col_preference)
            } else {
                buffer.below_offset(position, *col_preference)
            }
        }
    }
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

fn changed_byte_range(before: &str, after: &str) -> Option<(usize, usize)> {
    let head = before
        .bytes()
        .zip(after.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    if head == before.len() && head == after.len() {
        return None;
    }
    let tail = before.as_bytes()[head..]
        .iter()
        .rev()
        .zip(after.as_bytes()[head..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    Some((head, before.len() - tail))
}

fn new_cursor_offset(prev_value: &str, cursor_offset: usize, value: &str) -> usize {
    let has_same_head = value.len() >= cursor_offset
        && value.as_bytes()[..cursor_offset] == prev_value.as_bytes()[..cursor_offset];

    let tail_len = prev_value.len() - cursor_offset;
    let has_same_tail = value.len() >= tail_len
        && value.as_bytes()[value.len() - tail_len..] == prev_value.as_bytes()[cursor_offset..];

    if value.len() >= prev_value.len() && has_same_tail {
        // Keep the cursor anchored to the suffix when a parent normalizes the edited prefix.
        cursor_offset + (value.len() - prev_value.len())
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
        requested_cursor_offset: Option<usize>,
        uppercase: bool,
        reject_shrinking_changes: bool,
        reject_character: Option<char>,
        minimum_length: usize,
        multiline: bool,
    }

    #[component]
    fn MyComponent(mut hooks: Hooks, props: &MyComponentProps) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut value = hooks.use_state(|| props.initial_value.clone());
        let mut handle = hooks.use_ref_default::<TextInputHandle>();
        let requested_cursor_offset = props.requested_cursor_offset;
        let uppercase = props.uppercase;
        let reject_shrinking_changes = props.reject_shrinking_changes;
        let reject_character = props.reject_character;
        let minimum_length = props.minimum_length;
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
            View(height: if props.multiline { 3 } else { 1 }, width: 11, padding_left: 1) {
                TextInput(
                    has_focus: true,
                    value: value.to_string(),
                    on_change: move |new_value: String| {
                        let accept = new_value.len() >= minimum_length
                            && (!reject_shrinking_changes || new_value.len() >= value.read().len())
                            && reject_character.is_none_or(|c| !new_value.contains(c));
                        if accept {
                            value.set(if uppercase { new_value.to_uppercase() } else { new_value });
                        }
                    },
                    handle,
                    multiline: props.multiline,
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

    #[apply(test!)]
    async fn test_batched_cursor_movement_and_editing() {
        let frames: Vec<_> = element!(MyComponent(initial_value: "abc"))
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter([
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Left)),
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('x'))),
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('!'))),
            ])))
            .collect()
            .await;
        assert_eq!(frames.last().unwrap().to_string().trim(), "abx!c");
    }

    #[apply(test!)]
    async fn test_batched_deletions_at_text_boundaries() {
        for (movement, deletion, expected) in [
            (KeyCode::Home, KeyCode::Backspace, "!abc"),
            (KeyCode::End, KeyCode::Delete, "abc!"),
        ] {
            let events = [movement, deletion, KeyCode::Char('!')]
                .into_iter()
                .map(|code| TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, code)));
            let frames: Vec<_> = element!(MyComponent(initial_value: "abc"))
                .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                    events,
                )))
                .collect()
                .await;
            assert_eq!(frames.last().unwrap().to_string().trim(), expected);
        }
    }

    #[apply(test!)]
    async fn test_batched_navigation_uses_edited_utf8() {
        let frames: Vec<_> = element!(MyComponent(initial_value: "界a"))
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter([
                TerminalEvent::BracketedPaste("é".to_string()),
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Left)),
                TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, KeyCode::Char('!'))),
            ])))
            .collect()
            .await;
        assert_eq!(frames.last().unwrap().to_string().trim(), "界a!é");
    }

    #[apply(test!)]
    async fn test_delete_keeps_cursor_after_parent_redraw() {
        let (sender, events) = futures::channel::mpsc::unbounded();
        let mut app = element!(MyComponent(initial_value: "aaa"));
        let frames = app.mock_terminal_render_loop(MockTerminalConfig::with_events(events));
        futures::pin_mut!(frames);
        frames.next().await.unwrap();
        for code in [KeyCode::Home, KeyCode::Right, KeyCode::Delete] {
            sender
                .unbounded_send(TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, code)))
                .unwrap();
        }
        assert_eq!(frames.next().await.unwrap().to_string().trim(), "aa");
        sender
            .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                KeyEventKind::Press,
                KeyCode::Char('!'),
            )))
            .unwrap();
        assert_eq!(frames.next().await.unwrap().to_string().trim(), "a!a");
    }

    #[apply(test!)]
    async fn test_rejected_delete_preserves_cursor_after_redraw() {
        smol::future::or(
            async {
                let (sender, events) = futures::channel::mpsc::unbounded();
                let mut app = element!(MyComponent(
                    initial_value: "aaa",
                    reject_shrinking_changes: true,
                ));
                let frames = app.mock_terminal_render_loop(MockTerminalConfig::with_events(events));
                futures::pin_mut!(frames);
                frames.next().await.unwrap();
                for code in [KeyCode::Home, KeyCode::Right, KeyCode::Delete] {
                    sender
                        .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                            KeyEventKind::Press,
                            code,
                        )))
                        .unwrap();
                }
                assert_eq!(frames.next().await.unwrap().to_string().trim(), "aaa");
                sender
                    .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                        KeyEventKind::Press,
                        KeyCode::Char('!'),
                    )))
                    .unwrap();
                assert_eq!(frames.next().await.unwrap().to_string().trim(), "a!aa");
            },
            async {
                smol::Timer::after(std::time::Duration::from_secs(5)).await;
                panic!("controlled TextInput did not redraw");
            },
        )
        .await;
    }

    #[component]
    fn NavigationRejectionApp(
        mut hooks: Hooks,
        props: &MyComponentProps,
    ) -> impl Into<AnyElement<'static>> {
        let mut event_count = hooks.use_state(|| 0usize);
        hooks.use_terminal_events(move |_| event_count += 1);
        // A rejected update must yield a frame even when the input looks unchanged.
        element! {
            View(flex_direction: FlexDirection::Column) {
                MyComponent(
                    initial_value: props.initial_value.clone(),
                    reject_shrinking_changes: props.reject_shrinking_changes,
                    reject_character: props.reject_character,
                    multiline: props.multiline,
                )
                Text(content: event_count.to_string())
            }
        }
    }

    #[apply(test!)]
    async fn test_navigation_after_rejected_edit_survives_redraw() {
        smol::future::or(
            async {
                for (initial, edit, movement, multiline, next_movement, expected) in [
                    (
                        "abc",
                        KeyCode::Backspace,
                        KeyCode::Home,
                        false,
                        None,
                        "!abc",
                    ),
                    (
                        "abc",
                        KeyCode::Char('x'),
                        KeyCode::Home,
                        false,
                        None,
                        "!abc",
                    ),
                    ("", KeyCode::Char('x'), KeyCode::Home, false, None, "!"),
                    (
                        "abc",
                        KeyCode::Backspace,
                        KeyCode::Left,
                        false,
                        None,
                        "ab!c",
                    ),
                    (
                        "ab\ncdef",
                        KeyCode::Backspace,
                        KeyCode::Up,
                        true,
                        Some(KeyCode::Down),
                        "ab\ncdef!",
                    ),
                ] {
                    let (sender, events) = futures::channel::mpsc::unbounded();
                    let mut app = element!(NavigationRejectionApp(
                        initial_value: initial,
                        reject_shrinking_changes: true,
                        reject_character: Some('x'),
                        multiline,
                    ));
                    let frames =
                        app.mock_terminal_render_loop(MockTerminalConfig::with_events(events));
                    futures::pin_mut!(frames);
                    frames.next().await.unwrap();
                    for code in [edit, movement] {
                        sender
                            .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                                KeyEventKind::Press,
                                code,
                            )))
                            .unwrap();
                    }
                    let height = if multiline { 3 } else { 1 };
                    let frame = frames.next().await.unwrap();
                    assert_eq!(frame.get_text(1, 0, 10, height).trim_end(), initial);
                    if let Some(code) = next_movement {
                        sender
                            .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                                KeyEventKind::Press,
                                code,
                            )))
                            .unwrap();
                        frames.next().await.unwrap();
                    }
                    sender
                        .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                            KeyEventKind::Press,
                            KeyCode::Char('!'),
                        )))
                        .unwrap();
                    let frame = frames.next().await.unwrap();
                    assert_eq!(
                        frame.get_text(1, 0, 10, height).trim_end(),
                        expected,
                        "{edit:?} then {movement:?}"
                    );
                }
            },
            async {
                smol::Timer::after(std::time::Duration::from_secs(5)).await;
                panic!("controlled TextInput did not redraw");
            },
        )
        .await;
    }

    #[apply(test!)]
    async fn test_reconcile_batched_normalization_before_navigation() {
        let (sender, events) = futures::channel::mpsc::unbounded();
        let mut app = element!(MyComponent(initial_value: "AB", uppercase: true));
        let frames = app.mock_terminal_render_loop(MockTerminalConfig::with_events(events));
        futures::pin_mut!(frames);
        frames.next().await.unwrap();
        for code in [KeyCode::Left, KeyCode::Char('x'), KeyCode::Home] {
            sender
                .unbounded_send(TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, code)))
                .unwrap();
        }
        assert_eq!(frames.next().await.unwrap().to_string().trim(), "AXB");
        sender
            .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                KeyEventKind::Press,
                KeyCode::Char('!'),
            )))
            .unwrap();
        assert_eq!(frames.next().await.unwrap().to_string().trim(), "!AXB");
    }

    #[apply(test!)]
    async fn test_reconcile_batched_partial_deletion() {
        let (sender, events) = futures::channel::mpsc::unbounded();
        let mut app = element!(MyComponent(initial_value: "aaa", minimum_length: 2usize));
        let frames = app.mock_terminal_render_loop(MockTerminalConfig::with_events(events));
        futures::pin_mut!(frames);
        frames.next().await.unwrap();
        for code in [
            KeyCode::Home,
            KeyCode::Right,
            KeyCode::Delete,
            KeyCode::Delete,
        ] {
            sender
                .unbounded_send(TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, code)))
                .unwrap();
        }
        assert_eq!(frames.next().await.unwrap().to_string().trim(), "aa");
        sender
            .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                KeyEventKind::Press,
                KeyCode::Char('!'),
            )))
            .unwrap();
        assert_eq!(frames.next().await.unwrap().to_string().trim(), "a!a");
    }

    #[apply(test!)]
    async fn test_reconcile_batched_partial_insertion_and_navigation() {
        let (sender, events) = futures::channel::mpsc::unbounded();
        let mut app = element!(MyComponent(initial_value: "abc", reject_character: Some('x')));
        let frames = app.mock_terminal_render_loop(MockTerminalConfig::with_events(events));
        futures::pin_mut!(frames);
        frames.next().await.unwrap();
        for code in [KeyCode::Char('d'), KeyCode::Char('x'), KeyCode::Left] {
            sender
                .unbounded_send(TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, code)))
                .unwrap();
        }
        assert_eq!(frames.next().await.unwrap().to_string().trim(), "abcd");
        sender
            .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                KeyEventKind::Press,
                KeyCode::Char('!'),
            )))
            .unwrap();
        assert_eq!(frames.next().await.unwrap().to_string().trim(), "abc!d");
    }

    #[apply(test!)]
    async fn test_reconcile_normalized_edit_before_rejected_edit() {
        for (character, deletion, accepted, expected) in [
            ('x', KeyCode::Delete, "AXB", "AX!B"),
            ('ǰ', KeyCode::Delete, "AJ\u{030c}B", "AJ\u{030c}!B"),
            ('é', KeyCode::Backspace, "AÉB", "AÉ!B"),
        ] {
            let (sender, events) = futures::channel::mpsc::unbounded();
            let mut app = element!(MyComponent(
                initial_value: "AB",
                uppercase: true,
                reject_shrinking_changes: true,
            ));
            let frames = app.mock_terminal_render_loop(MockTerminalConfig::with_events(events));
            futures::pin_mut!(frames);
            frames.next().await.unwrap();
            for code in [KeyCode::Left, KeyCode::Char(character), deletion] {
                sender
                    .unbounded_send(TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, code)))
                    .unwrap();
            }
            assert_eq!(frames.next().await.unwrap().to_string().trim(), accepted);
            sender
                .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                    KeyEventKind::Press,
                    KeyCode::Char('!'),
                )))
                .unwrap();
            assert_eq!(frames.next().await.unwrap().to_string().trim(), expected);
        }
    }

    #[apply(test!)]
    async fn test_parent_normalization_keeps_caret_before_unchanged_suffix() {
        let (sender, events) = futures::channel::mpsc::unbounded();
        let mut app = element!(MyComponent(initial_value: "AB", uppercase: true));
        let frames = app.mock_terminal_render_loop(MockTerminalConfig::with_events(events));
        futures::pin_mut!(frames);
        frames.next().await.unwrap();
        for code in [KeyCode::Left, KeyCode::Char('x')] {
            sender
                .unbounded_send(TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, code)))
                .unwrap();
            frames.next().await.unwrap();
        }
        sender
            .unbounded_send(TerminalEvent::Key(KeyEvent::new(
                KeyEventKind::Press,
                KeyCode::Char('!'),
            )))
            .unwrap();
        assert_eq!(frames.next().await.unwrap().to_string().trim(), "AX!B");
    }

    #[apply(test!)]
    async fn test_batched_multiline_navigation_after_edits() {
        let events = [
            KeyCode::Char('a'),
            KeyCode::Char('b'),
            KeyCode::Enter,
            KeyCode::Char('c'),
            KeyCode::Char('d'),
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Home,
            KeyCode::Right,
            KeyCode::Delete,
            KeyCode::Backspace,
            KeyCode::End,
            KeyCode::Char('!'),
        ]
        .into_iter()
        .map(|code| TerminalEvent::Key(KeyEvent::new(KeyEventKind::Press, code)));
        let frames: Vec<_> = element!(MyMultilineComponent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                events,
            )))
            .collect()
            .await;
        let canvas = frames.last().unwrap();
        assert_eq!(canvas.get_text(1, 0, 2, 1), "ab");
        assert_eq!(canvas.get_text(1, 1, 1, 1), "!");
    }

    #[apply(test!)]
    async fn test_batched_shortcuts_ignore_releases_and_unhandled_modifiers() {
        let event = |kind, code, modifiers| {
            let mut key = KeyEvent::new(kind, code);
            key.modifiers = modifiers;
            TerminalEvent::Key(key)
        };
        let press = KeyEventKind::Press;
        let none = KeyModifiers::NONE;
        let control = KeyModifiers::CONTROL;
        let events = [
            event(press, KeyCode::Char('a'), control),
            event(press, KeyCode::Char('x'), none),
            event(KeyEventKind::Release, KeyCode::Char('a'), control),
            event(press, KeyCode::Char('y'), none),
            event(press, KeyCode::Char('z'), control),
            event(press, KeyCode::Char('e'), control),
            event(press, KeyCode::Char('q'), KeyModifiers::ALT),
            event(press, KeyCode::Esc, none),
            event(press, KeyCode::Char('!'), none),
        ];
        let frames: Vec<_> = element!(MyComponent(initial_value: "abc"))
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                events,
            )))
            .collect()
            .await;
        assert_eq!(frames.last().unwrap().to_string().trim(), "xyabc!");
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
