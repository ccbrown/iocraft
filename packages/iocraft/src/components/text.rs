use crate::{
    CanvasTextStyle, Color, Component, ComponentDrawer, ComponentUpdater, Covariant, Weight,
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
#[derive(Default, Covariant)]
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
}

/// `Text` is a component that renders a text string.
#[derive(Default)]
pub struct Text {
    style: CanvasTextStyle,
    content: String,
    wrap: TextWrap,
    align: TextAlign,
}

impl Text {
    fn wrap(
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

    fn align(content: String, align: TextAlign, width: usize) -> String {
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

impl Component for Text {
    type Props<'a> = TextProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self::default()
    }

    fn update(&mut self, props: &mut Self::Props<'_>, updater: &mut ComponentUpdater) {
        self.style = CanvasTextStyle {
            color: props.color,
            weight: props.weight,
            underline: props.decoration == TextDecoration::Underline,
        };
        self.content = props.content.clone();
        self.wrap = props.wrap;
        self.align = props.align;

        {
            let content = self.content.clone();
            let text_wrap = props.wrap;
            updater.set_measure_func(Box::new(move |known_size, available_space, _| {
                let content =
                    Self::wrap(&content, text_wrap, known_size.width, available_space.width);
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
            }));
        }
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
        drawer.canvas().set_text(0, 0, &content, self.style);
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

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
                Box(width: 14) {
                    Text(content: "this is a wrapping test")
                }
            }
            .to_string(),
            "this is a\nwrapping test\n"
        );

        assert_eq!(
            element! {
                Box(width: 15) {
                    Text(content: "this is an alignment test", align: TextAlign::Right)
                }
            }
            .to_string(),
            "     this is an\n alignment test\n"
        );

        assert_eq!(
            element! {
                Box(width: 15) {
                    Text(content: "this is an alignment test", align: TextAlign::Center)
                }
            }
            .to_string(),
            "  this is an\nalignment test\n"
        );
    }
}
