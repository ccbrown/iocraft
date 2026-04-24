use crate::style::{Color, Weight};
use crossterm::{
    csi,
    style::{Attribute, Colored},
};
use std::{
    env,
    fmt::{self, Display},
    io::{self, Write},
    sync::Once,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Clone, Debug, PartialEq)]
struct Character {
    value: String,
    style: CanvasTextStyle,
}

static mut HANDLES_VS16_INCORRECTLY: bool = false;
static INIT_HANDLES_VS16_INCORRECTLY: Once = Once::new();

// Some terminals incorrectly only advance the cursor one space for emoji with VS16, so we need to
// add whitespace to compensate.
//
// https://www.jeffquast.com/post/ucs-detect-test-results/
// https://darrenburns.net/posts/emoji-in-the-terminal/
//
// Windows and iTerm2 seem to do the right thing. We add exceptions below for the ones that don't.
// Hopefully one day we'll be able to remove this hack.
pub(crate) fn handles_vs16_incorrectly() -> bool {
    unsafe {
        INIT_HANDLES_VS16_INCORRECTLY.call_once(|| {
            HANDLES_VS16_INCORRECTLY = env::var("TERM_PROGRAM")
                .map(|s| s == "Apple_Terminal")
                .unwrap_or(false)
                || env::var("GNOME_TERMINAL_SCREEN").is_ok_and(|v| !v.is_empty())
        });
        HANDLES_VS16_INCORRECTLY
    }
}

impl Character {
    fn required_padding(&self) -> usize {
        if self.value.contains('\u{fe0f}') {
            if handles_vs16_incorrectly() {
                self.value.width() - 1
            } else {
                0
            }
        } else {
            0
        }
    }
}

/// Describes the style of text to be rendered via a [`Canvas`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CanvasTextStyle {
    /// The color of the text.
    pub color: Option<Color>,

    /// The weight of the text.
    pub weight: Weight,

    /// Whether the text is underlined.
    pub underline: bool,

    /// Whether the text is italicized.
    pub italic: bool,

    /// Whether the foreground and background colors should be inverted.
    pub invert: bool,
}

/// A single cell on a [`Canvas`], containing optional text and background color.
#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanvasCell {
    /// The background color of this cell, if set.
    pub background_color: Option<Color>,
    character: Option<Character>,
}

impl CanvasCell {
    /// Returns the text content of this cell, or `None` if empty.
    pub fn text(&self) -> Option<&str> {
        self.character.as_ref().map(|ch| ch.value.as_str())
    }

    /// Returns the text style of this cell, or `None` if the cell is empty.
    pub fn text_style(&self) -> Option<&CanvasTextStyle> {
        self.character.as_ref().map(|ch| &ch.style)
    }

    /// Returns `true` if the cell has no content and no background color.
    pub fn is_empty(&self) -> bool {
        self.background_color.is_none() && self.character.is_none()
    }
}

/// `Canvas` is the medium that output is drawn to before being rendered to the terminal or other
/// destinations.
///
/// Typical use of the library doesn't require direct interaction with this struct. It is primarily useful for two cases:
///
/// - When implementing low-level components, you'll need to utilize the `Canvas` drawing methods.
/// - When implementing unit tests for components, you may want to render to a `Canvas` for inspection.
#[derive(Clone, PartialEq)]
pub struct Canvas {
    width: usize,
    cells: Vec<Vec<CanvasCell>>,
}

impl Canvas {
    /// Constructs a new canvas with the given dimensions.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            cells: vec![vec![CanvasCell::default(); width]; height],
        }
    }

    /// Returns the width of the canvas.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the height of the canvas.
    pub fn height(&self) -> usize {
        self.cells.len()
    }

    /// Returns a reference to the cell at the given position, or `None` if
    /// out of bounds.
    pub fn cell(&self, x: usize, y: usize) -> Option<&CanvasCell> {
        self.cells.get(y).and_then(|row| row.get(x))
    }

    /// Extracts plain text from a rectangular region of the canvas.
    ///
    /// Each row within the region produces one line in the result, separated
    /// by newlines. Trailing whitespace on each line is trimmed. Out-of-bounds
    /// coordinates are clamped silently.
    pub fn get_text(&self, x: usize, y: usize, w: usize, h: usize) -> String {
        let mut lines = Vec::with_capacity(h);
        for row_idx in y..y + h {
            let Some(row) = self.cells.get(row_idx) else {
                lines.push(String::new());
                continue;
            };
            let start = x.min(row.len());
            let end = (x + w).min(row.len());
            let slice = &row[start..end];
            let last_non_empty = slice.iter().rposition(|cell| cell.character.is_some());
            let trim_end = match last_non_empty {
                Some(i) => i + 1,
                None => {
                    lines.push(String::new());
                    continue;
                }
            };
            let mut s = String::with_capacity(trim_end);
            for cell in &slice[..trim_end] {
                match cell.character.as_ref() {
                    Some(ch) => s.push_str(&ch.value),
                    None => s.push(' '),
                }
            }
            lines.push(s);
        }
        lines.join(
            "
",
        )
    }

    fn clear_text(&mut self, x: usize, y: usize, w: usize, h: usize) {
        for y in y..y + h {
            if let Some(row) = self.cells.get_mut(y) {
                for x in x..x + w {
                    if x < row.len() {
                        row[x].character = None;
                    }
                }
            }
        }
    }

    fn set_background_color(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        for y in y..y + h {
            if let Some(row) = self.cells.get_mut(y) {
                for x in x..x + w {
                    if x < row.len() {
                        row[x].background_color = Some(color);
                    }
                }
            }
        }
    }

    fn set_text_row_chars<I>(&mut self, mut x: usize, y: usize, chars: I, style: CanvasTextStyle)
    where
        I: IntoIterator<Item = char>,
    {
        // Divide the string up into characters, which may consist of multiple Unicode code points.
        let row = &mut self.cells[y];
        let mut buf = String::new();
        for c in chars.into_iter() {
            if x >= row.len() {
                break;
            }
            let width = c.width().unwrap_or(0);
            if width > 0 && !buf.is_empty() {
                row[x].character = Some(Character {
                    value: buf.clone(),
                    style,
                });
                x += buf.width().max(1);
                buf.clear();
            }
            buf.push(c);
        }
        if !buf.is_empty() && x < row.len() {
            row[x].character = Some(Character { value: buf, style });
        }
    }

    /// Gets a subview of the canvas for writing.
    pub fn subview_mut(
        &mut self,
        x: isize,
        y: isize,
        clip_x: isize,
        clip_y: isize,
        clip_width: usize,
        clip_height: usize,
    ) -> CanvasSubviewMut<'_> {
        CanvasSubviewMut {
            y,
            x,
            clip_x,
            clip_y,
            clip_width,
            clip_height,
            canvas: self,
        }
    }

    fn write_impl<W: Write>(
        &self,
        mut w: W,
        ansi: bool,
        omit_final_newline: bool,
    ) -> io::Result<()> {
        if ansi {
            write!(w, csi!("0m"))?;
        }

        let mut background_color = None;
        let mut text_style = CanvasTextStyle::default();

        for y in 0..self.cells.len() {
            let row = &self.cells[y];
            let last_non_empty = row.iter().rposition(|cell| !cell.is_empty());
            let row = &row[..last_non_empty.map_or(0, |i| i + 1)];
            let mut col = 0;
            let mut did_clear_line = false;
            while col < row.len() {
                let cell = &row[col];

                if ansi {
                    // For certain changes, we need to reset all attributes.
                    let mut needs_reset = false;
                    if let Some(c) = &cell.character {
                        if c.style.weight != text_style.weight && c.style.weight == Weight::Normal {
                            needs_reset = true;
                        }
                        if !c.style.underline && text_style.underline {
                            needs_reset = true;
                        }
                        if !c.style.italic && text_style.italic {
                            needs_reset = true;
                        }
                        if !c.style.invert && text_style.invert {
                            needs_reset = true;
                        }
                    } else if text_style.underline || text_style.invert {
                        needs_reset = true;
                    }
                    if needs_reset {
                        write!(w, csi!("0m"))?;
                        background_color = None;
                        text_style = CanvasTextStyle::default();
                    }

                    if let Some(c) = &cell.character {
                        if c.style.color != text_style.color {
                            write!(
                                w,
                                csi!("{}m"),
                                Colored::ForegroundColor(c.style.color.unwrap_or(Color::Reset))
                            )?;
                        }

                        if c.style.weight != text_style.weight {
                            match c.style.weight {
                                Weight::Bold => write!(w, csi!("{}m"), Attribute::Bold.sgr())?,
                                Weight::Normal => {}
                                Weight::Light => write!(w, csi!("{}m"), Attribute::Dim.sgr())?,
                            }
                        }

                        if c.style.underline && !text_style.underline {
                            write!(w, csi!("{}m"), Attribute::Underlined.sgr())?;
                        }

                        if c.style.italic && !text_style.italic {
                            write!(w, csi!("{}m"), Attribute::Italic.sgr())?;
                        }

                        if c.style.invert && !text_style.invert {
                            write!(w, csi!("{}m"), Attribute::Reverse.sgr())?;
                        }

                        text_style = c.style;
                    }
                }

                if let Some(c) = &cell.character {
                    col += c.value.width().max(1);
                } else {
                    col += 1;
                }

                if ansi && col >= self.width {
                    // go ahead and clear until end of line. we need to do this before writing
                    // the last character, because if we're at the end of the terminal row, the
                    // cursor won't change position and the last character would be erased
                    // if we did it later
                    // see: https://github.com/ccbrown/iocraft/issues/83

                    // make sure to reset the background before clearing
                    // see: https://github.com/ccbrown/iocraft/issues/142
                    if background_color.is_some() {
                        write!(w, csi!("{}m"), Colored::BackgroundColor(Color::Reset))?;
                        background_color = None;
                    }

                    write!(w, csi!("K"))?;
                    did_clear_line = true;
                }

                if ansi && cell.background_color != background_color {
                    write!(
                        w,
                        csi!("{}m"),
                        Colored::BackgroundColor(cell.background_color.unwrap_or(Color::Reset))
                    )?;
                    background_color = cell.background_color;
                }

                if let Some(c) = &cell.character {
                    write!(w, "{}{}", c.value, " ".repeat(c.required_padding()))?;
                } else {
                    w.write_all(b" ")?;
                }
            }
            if ansi {
                // if the background color is set, we need to reset it
                if background_color.is_some() {
                    write!(w, csi!("{}m"), Colored::BackgroundColor(Color::Reset))?;
                    background_color = None;
                }
                if !did_clear_line {
                    // clear until end of line
                    write!(w, csi!("K"))?;
                }
            }
            let is_final_line = y == self.cells.len() - 1;
            if !omit_final_newline || !is_final_line {
                if ansi {
                    if is_final_line {
                        write!(w, csi!("0m"))?;
                    }
                    // add a carriage return in case we're in raw mode
                    w.write_all(b"\r\n")?;
                } else {
                    w.write_all(b"\n")?;
                }
            }
        }
        if ansi && omit_final_newline {
            write!(w, csi!("0m"))?;
        }
        w.flush()?;
        Ok(())
    }

    /// Writes the canvas to the given writer with ANSI escape codes.
    pub fn write_ansi<W: Write>(&self, w: W) -> io::Result<()> {
        self.write_impl(w, true, false)
    }

    pub(crate) fn write_ansi_without_final_newline<W: Write>(&self, w: W) -> io::Result<()> {
        self.write_impl(w, true, true)
    }

    /// Writes the canvas to the given writer as unstyled text, without ANSI escape codes.
    pub fn write<W: Write>(&self, w: W) -> io::Result<()> {
        self.write_impl(w, false, false)
    }
}

impl Display for Canvas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = Vec::with_capacity(self.width * self.cells.len());
        self.write(&mut buf).unwrap();
        f.write_str(&String::from_utf8_lossy(&buf))?;
        Ok(())
    }
}

/// Represents a writeable region of a [`Canvas`]. All coordinates provided to functions of this
/// type are relative to the region's top-left corner.
pub struct CanvasSubviewMut<'a> {
    x: isize,
    y: isize,
    clip_x: isize,
    clip_y: isize,
    clip_width: usize,
    clip_height: usize,
    canvas: &'a mut Canvas,
}

impl CanvasSubviewMut<'_> {
    /// Returns a reference to a cell at the given **relative** subview position.
    ///
    /// Returns `None` if the resulting absolute position is out of bounds or
    /// outside the clip region.
    pub fn cell(&self, x: isize, y: isize) -> Option<&CanvasCell> {
        let abs_x = self.x + x;
        let abs_y = self.y + y;
        if abs_x < self.clip_x
            || abs_y < self.clip_y
            || abs_x < 0
            || abs_y < 0
            || abs_x >= self.clip_x + self.clip_width as isize
            || abs_y >= self.clip_y + self.clip_height as isize
        {
            return None;
        }
        self.canvas.cell(abs_x as usize, abs_y as usize)
    }

    /// Extracts plain text from a rectangular region using **relative** subview
    /// coordinates. The region is clamped to the clip bounds.
    pub fn get_text(&self, x: isize, y: isize, w: usize, h: usize) -> String {
        let mut left = self.x + x;
        let mut top = self.y + y;
        let mut right = left + w as isize;
        let mut bottom = top + h as isize;

        left = left.max(self.clip_x).max(0);
        top = top.max(self.clip_y).max(0);
        right = right.min(self.clip_x + self.clip_width as isize).max(0);
        bottom = bottom.min(self.clip_y + self.clip_height as isize).max(0);

        self.canvas.get_text(
            left as _,
            top as _,
            (right - left).max(0) as _,
            (bottom - top).max(0) as _,
        )
    }

    /// Fills the region with the given color.
    pub fn set_background_color(&mut self, x: isize, y: isize, w: usize, h: usize, color: Color) {
        let mut left = self.x + x;
        let mut top = self.y + y;
        let mut right = left + w as isize;
        let mut bottom = top + h as isize;

        left = left.max(self.clip_x).max(0);
        top = top.max(self.clip_y).max(0);
        right = right.min(self.clip_x + self.clip_width as isize).max(0);
        bottom = bottom.min(self.clip_y + self.clip_height as isize).max(0);

        self.canvas.set_background_color(
            left as _,
            top as _,
            (right - left).max(0) as _,
            (bottom - top).max(0) as _,
            color,
        );
    }

    /// Removes text from the region.
    pub fn clear_text(&mut self, x: isize, y: isize, w: usize, h: usize) {
        let mut left = self.x + x;
        let mut top = self.y + y;
        let mut right = left + w as isize;
        let mut bottom = top + h as isize;

        left = left.max(self.clip_x).max(0);
        top = top.max(self.clip_y).max(0);
        right = right.min(self.clip_x + self.clip_width as isize).max(0);
        bottom = bottom.min(self.clip_y + self.clip_height as isize).max(0);

        self.canvas.clear_text(
            left as _,
            top as _,
            (right - left).max(0) as _,
            (bottom - top).max(0) as _,
        );
    }

    /// Writes text to the region.
    pub fn set_text(&mut self, x: isize, y: isize, text: &str, style: CanvasTextStyle) {
        let mut x = self.x + x;
        let min_x = self.clip_x.max(0);
        let mut to_skip = 0;
        if x < min_x {
            to_skip = min_x - x;
            x = min_x;
        }
        let max_x = self.clip_x + self.clip_width as isize - 1;
        let horizontal_space = max_x - x + 1;
        let min_y = self.clip_y.max(0);
        let max_y = (self.clip_y + self.clip_height as isize).min(self.canvas.height() as _) - 1;
        let mut y = self.y + y;
        for line in text.lines() {
            if y >= min_y && y <= max_y {
                let mut skipped_width = 0;
                let mut taken_width = 0;
                self.canvas.set_text_row_chars(
                    x as usize,
                    y as usize,
                    line.chars()
                        .skip_while(|c| {
                            if skipped_width < to_skip {
                                skipped_width += c.width().unwrap_or(0) as isize;
                                true
                            } else {
                                false
                            }
                        })
                        .take_while(|c| {
                            if taken_width < horizontal_space {
                                taken_width += c.width().unwrap_or(0) as isize;
                                true
                            } else {
                                false
                            }
                        }),
                    style,
                );
            }
            y += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_canvas_background_color() {
        let mut canvas = Canvas::new(6, 3);
        assert_eq!(canvas.width(), 6);
        assert_eq!(canvas.height(), 3);

        canvas
            .subview_mut(2, 0, 2, 0, 3, 2)
            .set_background_color(0, 0, 5, 5, Color::Red);

        let mut actual = Vec::new();
        canvas.write_ansi(&mut actual).unwrap();

        let mut expected = Vec::new();
        write!(expected, csi!("0m")).unwrap();
        write!(expected, "  ").unwrap();
        write!(expected, csi!("{}m"), Colored::BackgroundColor(Color::Red)).unwrap();
        write!(expected, "   ").unwrap();
        write!(
            expected,
            csi!("{}m"),
            Colored::BackgroundColor(Color::Reset)
        )
        .unwrap();
        write!(expected, csi!("K")).unwrap();
        write!(expected, "\r\n").unwrap();
        write!(expected, "  ").unwrap();
        write!(expected, csi!("{}m"), Colored::BackgroundColor(Color::Red)).unwrap();
        write!(expected, "   ").unwrap();
        write!(
            expected,
            csi!("{}m"),
            Colored::BackgroundColor(Color::Reset)
        )
        .unwrap();
        write!(expected, csi!("K")).unwrap();
        write!(expected, "\r\n").unwrap();
        write!(expected, csi!("K")).unwrap();
        write!(expected, csi!("0m")).unwrap();
        write!(expected, "\r\n").unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_canvas_full_background_color() {
        let mut canvas = Canvas::new(6, 3);
        assert_eq!(canvas.width(), 6);
        assert_eq!(canvas.height(), 3);

        canvas
            .subview_mut(0, 0, 0, 0, 6, 6)
            .set_background_color(0, 0, 6, 6, Color::Red);

        let mut actual = Vec::new();
        canvas.write_ansi(&mut actual).unwrap();

        // the important thing here is that the background color is reset before each line is
        // cleared and before each newline
        // see: https://github.com/ccbrown/iocraft/issues/142

        let mut expected = Vec::new();

        // line 1
        write!(expected, csi!("0m")).unwrap();
        write!(expected, csi!("{}m"), Colored::BackgroundColor(Color::Red)).unwrap();
        write!(expected, "     ").unwrap();
        write!(
            expected,
            csi!("{}m"),
            Colored::BackgroundColor(Color::Reset)
        )
        .unwrap();
        write!(expected, csi!("K")).unwrap();
        write!(expected, csi!("{}m"), Colored::BackgroundColor(Color::Red)).unwrap();
        write!(expected, " ").unwrap();
        write!(
            expected,
            csi!("{}m"),
            Colored::BackgroundColor(Color::Reset)
        )
        .unwrap();
        write!(expected, "\r\n").unwrap();

        // line 2
        write!(expected, csi!("{}m"), Colored::BackgroundColor(Color::Red)).unwrap();
        write!(expected, "     ").unwrap();
        write!(
            expected,
            csi!("{}m"),
            Colored::BackgroundColor(Color::Reset)
        )
        .unwrap();
        write!(expected, csi!("K")).unwrap();
        write!(expected, csi!("{}m"), Colored::BackgroundColor(Color::Red)).unwrap();
        write!(expected, " ").unwrap();
        write!(
            expected,
            csi!("{}m"),
            Colored::BackgroundColor(Color::Reset)
        )
        .unwrap();
        write!(expected, "\r\n").unwrap();

        // line 3
        write!(expected, csi!("{}m"), Colored::BackgroundColor(Color::Red)).unwrap();
        write!(expected, "     ").unwrap();
        write!(
            expected,
            csi!("{}m"),
            Colored::BackgroundColor(Color::Reset)
        )
        .unwrap();
        write!(expected, csi!("K")).unwrap();
        write!(expected, csi!("{}m"), Colored::BackgroundColor(Color::Red)).unwrap();
        write!(expected, " ").unwrap();
        write!(
            expected,
            csi!("{}m"),
            Colored::BackgroundColor(Color::Reset)
        )
        .unwrap();
        write!(expected, csi!("0m")).unwrap();
        write!(expected, "\r\n").unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_canvas_text_styles() {
        let mut canvas = Canvas::new(100, 1);
        assert_eq!(canvas.width(), 100);
        assert_eq!(canvas.height(), 1);

        canvas
            .subview_mut(0, 0, 0, 0, 1, 1)
            .set_text(0, 0, ".", CanvasTextStyle::default());
        canvas.subview_mut(1, 0, 1, 0, 1, 1).set_text(
            0,
            0,
            ".",
            CanvasTextStyle {
                color: Some(Color::Red),
                weight: Weight::Bold,
                underline: true,
                ..Default::default()
            },
        );
        canvas.subview_mut(2, 0, 2, 0, 1, 1).set_text(
            0,
            0,
            ".",
            CanvasTextStyle {
                color: Some(Color::Red),
                weight: Weight::Bold,
                italic: true,
                ..Default::default()
            },
        );
        canvas.subview_mut(3, 0, 3, 0, 1, 1).set_text(
            0,
            0,
            ".",
            CanvasTextStyle {
                color: Some(Color::Red),
                weight: Weight::Bold,
                ..Default::default()
            },
        );
        canvas.subview_mut(4, 0, 4, 0, 1, 1).set_text(
            0,
            0,
            ".",
            CanvasTextStyle {
                color: Some(Color::Red),
                weight: Weight::Light,
                ..Default::default()
            },
        );
        canvas.subview_mut(5, 0, 5, 0, 1, 1).set_text(
            0,
            0,
            ".",
            CanvasTextStyle {
                color: Some(Color::Red),
                ..Default::default()
            },
        );
        canvas.subview_mut(6, 0, 6, 0, 1, 1).set_text(
            0,
            0,
            ".",
            CanvasTextStyle {
                color: Some(Color::Green),
                ..Default::default()
            },
        );
        canvas.subview_mut(7, 0, 7, 0, 1, 1).set_text(
            0,
            0,
            ".",
            CanvasTextStyle {
                color: Some(Color::Green),
                invert: true,
                ..Default::default()
            },
        );
        canvas.subview_mut(8, 0, 8, 0, 1, 1).set_text(
            0,
            0,
            ".",
            CanvasTextStyle {
                color: Some(Color::Green),
                ..Default::default()
            },
        );

        let mut actual = Vec::new();
        canvas.write_ansi(&mut actual).unwrap();

        let mut expected = Vec::new();
        write!(expected, csi!("0m")).unwrap();
        write!(expected, ".").unwrap();

        write!(expected, csi!("{}m"), Colored::ForegroundColor(Color::Red)).unwrap();
        write!(expected, csi!("{}m"), Attribute::Bold.sgr()).unwrap();
        write!(expected, csi!("{}m"), Attribute::Underlined.sgr()).unwrap();
        write!(expected, ".").unwrap();

        write!(expected, csi!("0m")).unwrap();
        write!(expected, csi!("{}m"), Colored::ForegroundColor(Color::Red)).unwrap();
        write!(expected, csi!("{}m"), Attribute::Bold.sgr()).unwrap();
        write!(expected, csi!("{}m"), Attribute::Italic.sgr()).unwrap();
        write!(expected, ".").unwrap();

        write!(expected, csi!("0m")).unwrap();
        write!(expected, csi!("{}m"), Colored::ForegroundColor(Color::Red)).unwrap();
        write!(expected, csi!("{}m"), Attribute::Bold.sgr()).unwrap();
        write!(expected, ".").unwrap();

        write!(expected, csi!("{}m"), Attribute::Dim.sgr()).unwrap();
        write!(expected, ".").unwrap();

        write!(expected, csi!("0m")).unwrap();
        write!(expected, csi!("{}m"), Colored::ForegroundColor(Color::Red)).unwrap();
        write!(expected, ".").unwrap();

        write!(
            expected,
            csi!("{}m"),
            Colored::ForegroundColor(Color::Green)
        )
        .unwrap();
        write!(expected, ".").unwrap();

        write!(expected, csi!("{}m"), Attribute::Reverse.sgr()).unwrap();
        write!(expected, ".").unwrap();

        write!(expected, csi!("0m")).unwrap();
        write!(
            expected,
            csi!("{}m"),
            Colored::ForegroundColor(Color::Green)
        )
        .unwrap();
        write!(expected, ".").unwrap();

        write!(expected, csi!("K")).unwrap();
        write!(expected, csi!("0m")).unwrap();
        write!(expected, "\r\n").unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_canvas_text_clipping() {
        let mut canvas = Canvas::new(10, 5);
        assert_eq!(canvas.width(), 10);
        assert_eq!(canvas.height(), 5);

        canvas.subview_mut(2, 2, 2, 2, 4, 2).set_text(
            -2,
            -1,
            "line 1\nline 2\nline 3\nline 4",
            CanvasTextStyle::default(),
        );

        let actual = canvas.to_string();
        assert_eq!(actual, "\n\n  ne 2\n  ne 3\n\n");
    }

    #[test]
    fn test_canvas_text_clearing() {
        let mut canvas = Canvas::new(10, 1);
        canvas
            .subview_mut(0, 0, 0, 0, 10, 1)
            .set_text(0, 0, "hello!", CanvasTextStyle::default());
        assert_eq!(canvas.to_string(), "hello!\n");

        canvas.subview_mut(0, 0, 0, 0, 10, 1).clear_text(0, 0, 3, 1);
        assert_eq!(canvas.to_string(), "   lo!\n");
    }

    #[test]
    fn test_write_ansi_without_final_newline() {
        let mut canvas = Canvas::new(10, 3);

        canvas
            .subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 0, "hello!", CanvasTextStyle::default());

        let mut actual = Vec::new();
        canvas
            .write_ansi_without_final_newline(&mut actual)
            .unwrap();

        let mut expected = Vec::new();
        write!(expected, csi!("0m")).unwrap();
        write!(expected, "hello!").unwrap();
        write!(expected, csi!("K")).unwrap();
        write!(expected, "\r\n").unwrap();
        write!(expected, csi!("K")).unwrap();
        write!(expected, "\r\n").unwrap();
        write!(expected, csi!("K")).unwrap();
        write!(expected, csi!("0m")).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_ansi_erase_for_full_rows() {
        let mut canvas = Canvas::new(10, 1);

        canvas.subview_mut(0, 0, 0, 0, 10, 1).set_text(
            0,
            0,
            "1234512345",
            CanvasTextStyle::default(),
        );

        let mut actual = Vec::new();
        canvas.write_ansi(&mut actual).unwrap();

        let mut expected = Vec::new();
        write!(expected, csi!("0m")).unwrap();
        write!(expected, "123451234").unwrap();
        write!(expected, csi!("K")).unwrap();
        write!(expected, "5").unwrap();
        write!(expected, csi!("0m")).unwrap();
        write!(expected, "\r\n").unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_cell_read() {
        let mut canvas = Canvas::new(10, 3);
        canvas
            .subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 0, "hello", CanvasTextStyle::default());
        assert_eq!(canvas.cell(0, 0).and_then(|c| c.text()), Some("h"));
        assert_eq!(canvas.cell(4, 0).and_then(|c| c.text()), Some("o"));
        assert_eq!(canvas.cell(5, 0).and_then(|c| c.text()), None);
        assert_eq!(canvas.cell(99, 99), None);
    }

    #[test]
    fn test_get_text_single_row() {
        let mut canvas = Canvas::new(10, 3);
        let mut sv = canvas.subview_mut(0, 0, 0, 0, 10, 3);
        sv.set_text(0, 0, "hello", CanvasTextStyle::default());
        sv.set_text(2, 1, "ab", CanvasTextStyle::default());
        drop(sv);
        assert_eq!(canvas.get_text(0, 0, 10, 1), "hello");
        assert_eq!(canvas.get_text(0, 1, 10, 1), "  ab");
        assert_eq!(canvas.get_text(0, 2, 10, 1), "");
    }

    #[test]
    fn test_get_text_multi_row() {
        let mut canvas = Canvas::new(10, 3);
        let mut sv = canvas.subview_mut(0, 0, 0, 0, 10, 3);
        sv.set_text(0, 0, "line one", CanvasTextStyle::default());
        sv.set_text(0, 1, "line two", CanvasTextStyle::default());
        drop(sv);
        assert_eq!(
            canvas.get_text(0, 0, 10, 3),
            "line one
line two
"
        );
    }

    #[test]
    fn test_get_text_partial_row() {
        let mut canvas = Canvas::new(10, 1);
        canvas
            .subview_mut(0, 0, 0, 0, 10, 1)
            .set_text(0, 0, "abcdef", CanvasTextStyle::default());
        assert_eq!(canvas.get_text(2, 0, 3, 1), "cde");
    }

    #[test]
    fn test_cell_text_style() {
        let mut canvas = Canvas::new(10, 1);
        let style = CanvasTextStyle {
            weight: Weight::Bold,
            invert: true,
            ..Default::default()
        };
        canvas
            .subview_mut(0, 0, 0, 0, 10, 1)
            .set_text(0, 0, "hi", style);
        let cell = canvas.cell(0, 0).unwrap();
        let ts = cell.text_style().unwrap();
        assert_eq!(ts.weight, Weight::Bold);
        assert!(ts.invert);
        // Empty cell returns None.
        assert!(canvas.cell(5, 0).unwrap().text_style().is_none());
    }

    #[test]
    fn test_cell_is_empty() {
        let mut canvas = Canvas::new(5, 1);
        assert!(canvas.cell(0, 0).unwrap().is_empty());
        canvas
            .subview_mut(0, 0, 0, 0, 5, 1)
            .set_text(0, 0, "a", CanvasTextStyle::default());
        assert!(!canvas.cell(0, 0).unwrap().is_empty());
    }

    #[test]
    fn test_subview_cell_relative_coords() {
        let mut canvas = Canvas::new(10, 5);
        // Subview at offset (2, 1) with clip matching subview area
        let mut sv = canvas.subview_mut(2, 1, 2, 1, 6, 3);
        sv.set_text(0, 0, "abc", CanvasTextStyle::default());
        // Read back via subview using relative coordinates
        assert_eq!(sv.cell(0, 0).and_then(|c| c.text()), Some("a"));
        assert_eq!(sv.cell(2, 0).and_then(|c| c.text()), Some("c"));
        // Out of clip bounds → None
        assert_eq!(sv.cell(-1, 0), None);
        assert_eq!(sv.cell(6, 0), None);
        assert_eq!(sv.cell(0, -1), None);
        assert_eq!(sv.cell(0, 3), None);
    }

    #[test]
    fn test_subview_get_text_relative_coords() {
        let mut canvas = Canvas::new(10, 5);
        let mut sv = canvas.subview_mut(2, 1, 2, 1, 6, 3);
        sv.set_text(0, 0, "hello", CanvasTextStyle::default());
        sv.set_text(0, 1, "world", CanvasTextStyle::default());
        // Read back relative to subview origin
        assert_eq!(sv.get_text(0, 0, 6, 1), "hello");
        assert_eq!(sv.get_text(0, 0, 6, 2), "hello\nworld");
    }
}
