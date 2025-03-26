use crate::{
    render::MeasureFunc, CanvasTextStyle, Color, Component, ComponentDrawer, ComponentUpdater,
    Hooks, Props, Weight,
};
use taffy::{AvailableSpace, Size};
use unicode_width::UnicodeWidthStr;

/// The text wrapping behavior of a [`Text`] component.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum TextWrap {
    /// Text is wrapped at appropriate characters to minimize overflow. This is the default.
    #[default]
    Wrap,
    /// Text is not wrapped, and may overflow the bounds of the component.
    NoWrap,
}

/// The text alignment of a [`Text`] component.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum TextAlign {
    /// Text is aligned to the left. This is the default.
    #[default]
    Left,
    /// Text is aligned to the right.
    Right,
    /// Text is aligned to the center.
    Center,
}

/// The text decoration of a [`Text`] component.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum TextDecoration {
    /// No text decoration. This is the default.
    #[default]
    None,
    /// The text is underlined.
    Underline,
}

/// The props which can be passed to the [`Text`] component.
#[non_exhaustive]
#[derive(Default, Props)]
pub struct TextProps {
    /// The color to make the text.
    pub color: Option<Color>,

    /// The content of the text.
    pub content: String,

    /// The weight of the text.
    pub weight: Weight,

    /// The text wrapping behavior.
    pub wrap: TextWrap,

    /// The text alignment.
    pub align: TextAlign,

    /// The text decoration.
    pub decoration: TextDecoration,

    /// Whether to italicize the text.
    pub italic: bool,
}

/// `Text` is a component that renders a text string.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # fn my_element() -> impl Into<AnyElement<'static>> {
/// element! {
///     Text(content: "Hello!")
/// }
/// # }
/// ```
#[derive(Default)]
pub struct Text {
    style: CanvasTextStyle,
    content: String,
    wrap: TextWrap,
    align: TextAlign,
}

impl Text {
    pub(crate) fn measure_func(content: String, text_wrap: TextWrap) -> MeasureFunc {
        Box::new(move |known_size, available_space, _| {
            let content = Text::wrap(&content, text_wrap, known_size.width, available_space.width);
            let mut max_width = 0;
            let mut num_lines = 0;
            for line in content.lines() {
                max_width = max_width.max(line.width());
                num_lines += 1;
            }
            Size {
                width: max_width as _,
                height: num_lines.max(1) as _,
            }
        })
    }

    pub(crate) fn wrap(
        content: &str,
        text_wrap: TextWrap,
        known_width: Option<f32>,
        available_width: AvailableSpace,
    ) -> String {
        match text_wrap {
            TextWrap::Wrap => match known_width {
                Some(w) => textwrap::fill(content, w as usize),
                None => match available_width {
                    AvailableSpace::Definite(w) => textwrap::fill(content, w as usize),
                    AvailableSpace::MaxContent => content.to_string(),
                    AvailableSpace::MinContent => textwrap::fill(content, 1),
                },
            },
            TextWrap::NoWrap => content.to_string(),
        }
    }

    pub(crate) fn align(content: String, align: TextAlign, width: usize) -> String {
        match align {
            TextAlign::Left => content,
            TextAlign::Right => content
                .lines()
                .map(|line| {
                    let padding = width - line.width();
                    format!("{:width$}{}", "", line, width = padding)
                })
                .collect::<Vec<_>>()
                .join("\n"),
            TextAlign::Center => {
                let padding = width / 2;
                content
                    .lines()
                    .map(|line| {
                        let padding = padding - line.width() / 2;
                        format!("{:width$}{}", "", line, width = padding)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }
}

pub(crate) struct TextDrawer<'a, 'b> {
    x: isize,
    y: isize,
    drawer: &'a mut ComponentDrawer<'b>,
    line_encountered_non_whitespace: bool,
    skip_leading_whitespace: bool,
}

impl<'a, 'b> TextDrawer<'a, 'b> {
    pub fn new(drawer: &'a mut ComponentDrawer<'b>, skip_leading_whitespace: bool) -> Self {
        TextDrawer {
            x: 0,
            y: 0,
            drawer,
            line_encountered_non_whitespace: false,
            skip_leading_whitespace,
        }
    }

    pub fn append_lines<'c>(
        &mut self,
        lines: impl IntoIterator<Item = &'c str>,
        style: CanvasTextStyle,
    ) {
        let mut lines = lines.into_iter().peekable();
        while let Some(mut line) = lines.next() {
            if self.skip_leading_whitespace && !self.line_encountered_non_whitespace {
                let to_skip = line
                    .chars()
                    .position(|c| !c.is_whitespace())
                    .unwrap_or(line.len());
                let (whitespace, remaining) = line.split_at(to_skip);
                self.x += whitespace.width() as isize;
                line = remaining;
                if !line.is_empty() {
                    self.line_encountered_non_whitespace = true;
                }
            }
            self.drawer.canvas().set_text(self.x, self.y, line, style);
            if lines.peek().is_some() {
                self.y += 1;
                self.x = 0;
                self.line_encountered_non_whitespace = false;
            } else {
                self.x += line.width() as isize;
            }
        }
    }
}

impl Component for Text {
    type Props<'a> = TextProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self::default()
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        self.style = CanvasTextStyle {
            color: props.color,
            weight: props.weight,
            underline: props.decoration == TextDecoration::Underline,
            italic: props.italic,
        };
        self.content = props.content.clone();
        self.wrap = props.wrap;
        self.align = props.align;
        updater.set_measure_func(Self::measure_func(self.content.clone(), props.wrap));
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_>) {
        let width = drawer.layout().size.width;
        let content = Self::wrap(
            &self.content,
            self.wrap,
            None,
            AvailableSpace::Definite(width),
        );
        let content = Self::align(content, self.align, width as _);
        let mut drawer = TextDrawer::new(drawer, self.align != TextAlign::Left);
        drawer.append_lines(content.lines(), self.style);
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crossterm::{csi, style::Attribute};
    use std::io::Write;

    #[test]
    fn test_text() {
        assert_eq!(element!(Text).to_string(), "\n");

        assert_eq!(element!(Text(content: "foo")).to_string(), "foo\n");

        assert_eq!(
            element!(Text(content: "foo\nbar")).to_string(),
            "foo\nbar\n"
        );

        assert_eq!(element!(Text(content: "😀")).to_string(), "😀\n");

        assert_eq!(
            element! {
                View(width: 14) {
                    Text(content: "this is a wrapping test")
                }
            }
            .to_string(),
            "this is a\nwrapping test\n"
        );

        assert_eq!(
            element! {
                View(width: 15) {
                    Text(content: "this is an alignment test", align: TextAlign::Right)
                }
            }
            .to_string(),
            "     this is an\n alignment test\n"
        );

        assert_eq!(
            element! {
                View(width: 15) {
                    Text(content: "this is an alignment test", align: TextAlign::Center)
                }
            }
            .to_string(),
            "  this is an\nalignment test\n"
        );

        // Make sure that when the text is not left-aligned, leading whitespace is not underlined.
        {
            let canvas = element! {
                View(width: 16) {
                    Text(content: "this is an alignment test", align: TextAlign::Center, decoration: TextDecoration::Underline)
                }
            }
            .render(None);
            let mut actual = Vec::new();
            canvas.write_ansi(&mut actual).unwrap();

            let mut expected = Vec::new();
            write!(expected, csi!("0m")).unwrap();
            write!(expected, "   ").unwrap();
            write!(expected, csi!("{}m"), Attribute::Underlined.sgr()).unwrap();
            write!(expected, "this is an").unwrap();
            write!(expected, csi!("K")).unwrap();
            write!(expected, "\r\n").unwrap();
            write!(expected, csi!("0m")).unwrap();
            write!(expected, " ").unwrap();
            write!(expected, csi!("{}m"), Attribute::Underlined.sgr()).unwrap();
            write!(expected, "alignment test").unwrap();
            write!(expected, csi!("K")).unwrap();
            write!(expected, csi!("0m")).unwrap();
            write!(expected, "\r\n").unwrap();

            assert_eq!(actual, expected);
        }
    }
}
