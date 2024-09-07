use crate::{Color, Weight};
use crossterm::{
    csi,
    style::{Attribute, Attributes, Colored},
};
use std::io::{self, Write};

#[derive(Clone)]
struct Character {
    value: char,
    style: TextStyle,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextStyle {
    pub color: Option<Color>,
    pub weight: Weight,
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

pub struct Canvas {
    width: usize,
    cells: Vec<Vec<Cell>>,
}

impl Canvas {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            cells: Vec::new(),
        }
    }

    fn row_mut(&mut self, row: usize) -> &mut Vec<Cell> {
        while row >= self.cells.len() {
            self.cells.push(vec![Cell::default(); self.width]);
        }
        &mut self.cells[row]
    }

    fn set_background_color(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        for y in y..y + h {
            let row = self.row_mut(y);
            for x in x..x + w {
                if x < row.len() {
                    row[x].background_color = Some(color);
                }
            }
        }
    }

    fn set_text_chars<I>(&mut self, x: usize, y: usize, chars: I, style: TextStyle)
    where
        I: IntoIterator<Item = char>,
    {
        let row = self.row_mut(y);
        for (i, c) in chars.into_iter().enumerate() {
            if x + i < row.len() {
                row[x + i].character = Some(Character { value: c, style });
            }
        }
    }

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

    pub fn write_ansi<W: Write>(&self, mut w: W) -> io::Result<()> {
        let mut background_color = None;
        let mut text_style = TextStyle::default();
        for row in &self.cells {
            let last_non_empty = row.iter().rposition(|cell| !cell.is_empty());
            for cell in row.iter().take(last_non_empty.map_or(row.len(), |i| i + 1)) {
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
                        let mut attrs = Attributes::default();
                        match c.style.weight {
                            Weight::Bold => attrs.set(Attribute::Bold),
                            Weight::Normal => attrs.set(Attribute::Reset),
                            Weight::Light => attrs.set(Attribute::Dim),
                        }
                        for attr in Attribute::iterator() {
                            if attrs.has(attr) {
                                write!(w, csi!("{}m"), attr.sgr())?;
                            }
                        }
                    }

                    text_style = c.style;
                }

                if let Some(c) = &cell.character {
                    write!(w, "{}", c.value)?;
                } else {
                    w.write(b" ")?;
                }
            }
            w.write(b"\n")?;
        }
        w.flush()?;
        Ok(())
    }
}

pub struct CanvasSubviewMut<'a> {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    clip: bool,
    canvas: &'a mut Canvas,
}

impl<'a> CanvasSubviewMut<'a> {
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

    pub fn set_text(&mut self, x: isize, y: isize, text: &str, style: TextStyle) {
        if self.clip && y < 0 || y >= self.height as isize {
            return;
        }
        let y = self.y as isize + y;
        if y < 0 {
            return;
        }
        let mut x = self.x as isize + x;
        let min_x = if self.clip { self.x as isize } else { 0 };
        let mut to_skip = 0;
        if x < min_x {
            to_skip = min_x - x;
            x = min_x;
        }
        let max_x = if self.clip {
            (self.x + self.width - 1) as isize
        } else {
            self.canvas.width as isize - 1
        };
        let space = max_x - x + 1;
        self.canvas.set_text_chars(
            x as usize,
            y as usize,
            text.chars().skip(to_skip as _).take(space as _),
            style,
        );
    }
}