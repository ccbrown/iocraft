use crate::style::{Color, Weight};
use crossterm::{
    csi,
    style::{Attribute, Colored},
};
use std::{
    fmt::{self, Display},
    io::{self, Write},
};
use unicode_width::UnicodeWidthChar;

#[derive(Clone)]
struct Character {
    value: char,
    style: CanvasTextStyle,
}

/// Describes the style of text to be rendered via a [`Canvas`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CanvasTextStyle {
    /// The color of the text.
    pub color: Option<Color>,

    /// The weight of the text.
    pub weight: Weight,

    /// Whether the text is underlined.
    pub underline: bool,
}

#[derive(Clone, Default)]
struct Cell {
    background_color: Option<Color>,
    character: Option<Character>,
}

impl Cell {
    fn is_empty(&self) -> bool {
        self.background_color.is_none() && self.character.is_none()
    }
}

/// Canvas is a low-level abstraction for rendering output. Most users of the library will not need
/// to use it directly. However, it is used by low level component implementations and can be used
/// to store and copy their output.
pub struct Canvas {
    width: usize,
    cells: Vec<Vec<Cell>>,
}

impl Canvas {
    /// Constructs a new canvas with the given dimensions.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            cells: vec![vec![Cell::default(); width]; height],
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

    fn set_background_color(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        for y in y..y + h {
            let row = &mut self.cells[y];
            for x in x..x + w {
                if x < row.len() {
                    row[x].background_color = Some(color);
                }
            }
        }
    }

    fn set_text_row_chars<I>(&mut self, mut x: usize, y: usize, chars: I, style: CanvasTextStyle)
    where
        I: IntoIterator<Item = char>,
    {
        let row = &mut self.cells[y];
        for c in chars.into_iter() {
            if x >= row.len() {
                break;
            }
            row[x].character = Some(Character { value: c, style });
            x += c.width().unwrap_or(0);
        }
    }

    /// Gets a subview of the canvas for writing.
    pub fn subview_mut(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        clip: bool,
    ) -> CanvasSubviewMut {
        CanvasSubviewMut {
            y,
            x,
            width,
            height,
            clip,
            canvas: self,
        }
    }

    fn write_impl<W: Write>(&self, mut w: W, ansi: bool) -> io::Result<()> {
        if ansi {
            write!(w, csi!("0m"))?;
        }

        let mut background_color = None;
        let mut text_style = CanvasTextStyle::default();

        for row in &self.cells {
            let last_non_empty = row.iter().rposition(|cell| !cell.is_empty());
            let row = &row[..last_non_empty.map_or(0, |i| i + 1)];
            let mut col = 0;
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
                    } else if text_style.underline {
                        needs_reset = true;
                    }
                    if needs_reset {
                        write!(w, csi!("0m"))?;
                        background_color = None;
                        text_style = CanvasTextStyle::default();
                    }

                    if cell.background_color != background_color {
                        write!(
                            w,
                            csi!("{}m"),
                            Colored::BackgroundColor(cell.background_color.unwrap_or(Color::Reset))
                        )?;
                        background_color = cell.background_color;
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

                        text_style = c.style;
                    }
                }

                if let Some(c) = &cell.character {
                    write!(w, "{}", c.value)?;
                    col += c.value.width().unwrap_or(0);
                } else {
                    w.write_all(b" ")?;
                    col += 1;
                }
            }
            if ansi {
                // clear until end of line
                write!(w, csi!("K"))?;
                // add a carriage return in case we're in raw mode
                w.write_all(b"\r\n")?;
            } else {
                w.write_all(b"\n")?;
            }
        }
        if ansi {
            write!(w, csi!("0m"))?;
        }
        w.flush()?;
        Ok(())
    }

    /// Writes the canvas to the given writer with ANSI escape codes.
    pub fn write_ansi<W: Write>(&self, w: W) -> io::Result<()> {
        self.write_impl(w, true)
    }

    /// Writes the canvas to the given writer as unstyled text, without ANSI escape codes.
    pub fn write<W: Write>(&self, w: W) -> io::Result<()> {
        self.write_impl(w, false)
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
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    clip: bool,
    canvas: &'a mut Canvas,
}

impl<'a> CanvasSubviewMut<'a> {
    /// Fills the region with the given color.
    pub fn set_background_color(&mut self, x: isize, y: isize, w: usize, h: usize, color: Color) {
        let mut left = self.x as isize + x;
        let mut top = self.y as isize + y;
        let mut right = left + w as isize;
        let mut bottom = top + h as isize;
        if self.clip {
            left = left.max(self.x as isize);
            top = top.max(self.y as isize);
            right = right.min((self.x + self.width) as isize);
            bottom = bottom.min((self.y + self.height) as isize);
        }
        self.canvas.set_background_color(
            left as _,
            top as _,
            (right - left) as _,
            (bottom - top) as _,
            color,
        );
    }

    /// Writes text to the region.
    pub fn set_text(&mut self, x: isize, mut y: isize, text: &str, style: CanvasTextStyle) {
        let mut x = self.x as isize + x;
        let min_x = if self.clip { self.x as isize } else { 0 };
        let mut to_skip = 0;
        if x < min_x {
            to_skip = min_x - x;
            x = min_x;
        }
        let max_x = if self.clip {
            (self.x + self.width) as isize - 1
        } else {
            self.canvas.width as isize - 1
        };
        let horizontal_space = max_x - x + 1;
        for line in text.lines() {
            if !self.clip || (y >= 0 && y < self.height as isize) {
                let y = self.y as isize + y;
                if y >= 0 && y < self.canvas.height() as _ {
                    self.canvas.set_text_row_chars(
                        x as usize,
                        y as usize,
                        line.chars().skip(to_skip as _).take(horizontal_space as _),
                        style,
                    );
                }
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
            .subview_mut(2, 0, 3, 2, true)
            .set_background_color(0, 0, 5, 5, Color::Red);

        let mut actual = Vec::new();
        canvas.write_ansi(&mut actual).unwrap();

        let mut expected = Vec::new();
        write!(expected, csi!("0m")).unwrap();
        write!(expected, "  ").unwrap();
        write!(expected, csi!("{}m"), Colored::BackgroundColor(Color::Red)).unwrap();
        write!(expected, "   ").unwrap();
        write!(expected, csi!("K")).unwrap();
        write!(expected, "\r\n").unwrap();
        write!(
            expected,
            csi!("{}m"),
            Colored::BackgroundColor(Color::Reset)
        )
        .unwrap();
        write!(expected, "  ").unwrap();
        write!(expected, csi!("{}m"), Colored::BackgroundColor(Color::Red)).unwrap();
        write!(expected, "   ").unwrap();
        write!(expected, csi!("K")).unwrap();
        write!(expected, "\r\n").unwrap();
        write!(expected, csi!("K")).unwrap();
        write!(expected, "\r\n").unwrap();
        write!(expected, csi!("0m")).unwrap();

        assert_eq!(actual, expected);
    }
}
