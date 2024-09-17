use crate::{
    CanvasTextStyle, Color, Component, ComponentRenderer, ComponentUpdater, Covariant, Weight,
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
}

/// `Text` is a component that renders a text string.
#[derive(Default)]
pub struct Text {
    style: CanvasTextStyle,
    content: String,
    wrap: TextWrap,
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
                Some(w) => textwrap::fill(&content, w as usize),
                None => match available_width {
                    AvailableSpace::Definite(w) => textwrap::fill(&content, w as usize),
                    AvailableSpace::MaxContent => content.to_string(),
                    AvailableSpace::MinContent => textwrap::fill(&content, 1),
                },
            },
            TextWrap::NoWrap => content.to_string(),
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
        };
        self.content = props.content.clone();
        self.wrap = props.wrap;

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

    fn render(&mut self, renderer: &mut ComponentRenderer<'_>) {
        let content = Self::wrap(
            &self.content,
            self.wrap,
            None,
            AvailableSpace::Definite(renderer.layout().size.width),
        );
        renderer.canvas().set_text(0, 0, &content, self.style);
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
    }
}
