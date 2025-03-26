use crate::{
    components::text::{Text, TextAlign, TextDecoration, TextWrap},
    CanvasTextStyle, Color, Component, ComponentDrawer, ComponentUpdater, Hooks, Props, Weight,
};
use taffy::AvailableSpace;
use unicode_width::UnicodeWidthStr;

/// A section of text in a [`MixedText`] component.
#[non_exhaustive]
#[derive(Default, Clone)]
pub struct MixedTextContent {
    /// The text to display.
    pub text: String,

    /// The color to make the text.
    pub color: Option<Color>,

    /// The weight of the text.
    pub weight: Weight,

    /// The text decoration.
    pub decoration: TextDecoration,
}

impl MixedTextContent {
    /// Creates a new [`MixedTextContent`] with the given text.
    pub fn new<S: ToString>(text: S) -> Self {
        Self {
            text: text.to_string(),
            ..Default::default()
        }
    }

    /// Returns a new [`MixedTextContent`] with the given color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Returns a new [`MixedTextContent`] with the given weight.
    pub fn weight(mut self, weight: Weight) -> Self {
        self.weight = weight;
        self
    }

    /// Returns a new [`MixedTextContent`] with the given text decoration.
    pub fn decoration(mut self, decoration: TextDecoration) -> Self {
        self.decoration = decoration;
        self
    }
}

/// The props which can be passed to the [`MixedText`] component.
#[non_exhaustive]
#[derive(Default, Props)]
pub struct MixedTextProps {
    /// The contents of the text.
    pub contents: Vec<MixedTextContent>,

    /// The text wrapping behavior.
    pub wrap: TextWrap,

    /// The text alignment.
    pub align: TextAlign,
}

/// `MixedText` is a component that renders a text string containing a mix of styles.
///
/// If you want to render a text string with a single style, use the [`Text`] component instead.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # fn my_element() -> impl Into<AnyElement<'static>> {
/// element! {
///     View(
///         border_style: BorderStyle::Round,
///         border_color: Color::Blue,
///         width: 30,
///     ) {
///         MixedText(align: TextAlign::Center, contents: vec![
///             MixedTextContent::new("Hello, world!").color(Color::Red).weight(Weight::Bold),
///             MixedTextContent::new(" Lorem ipsum odor amet, consectetuer adipiscing elit.").color(Color::Green),
///         ])
///     }
/// }
/// # }
/// ```
#[derive(Default)]
pub struct MixedText {
    plaintext: String,
    contents: Vec<MixedTextContent>,
    wrap: TextWrap,
    align: TextAlign,
}

impl Component for MixedText {
    type Props<'a> = MixedTextProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self::default()
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        // Join the text contents using a zero-width space for wrapping. We'll later split the text
        // back into regions using those spaces.
        self.plaintext = props
            .contents
            .iter()
            .map(|content| content.text.replace("\u{200B}", ""))
            .collect::<Vec<_>>()
            .join("\u{200B}");
        self.contents = props.contents.clone();
        self.wrap = props.wrap;
        self.align = props.align;
        updater.set_measure_func(Text::measure_func(self.plaintext.clone(), props.wrap));
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_>) {
        let width = drawer.layout().size.width;
        let content = Text::wrap(
            &self.plaintext,
            self.wrap,
            None,
            AvailableSpace::Definite(width),
        );
        let content = Text::align(content, self.align, width as _);
        let mut x = 0;
        let mut y = 0;
        for (text, content) in content.split('\u{200B}').zip(&self.contents) {
            let style = CanvasTextStyle {
                color: content.color,
                weight: content.weight,
                underline: content.decoration == TextDecoration::Underline,
            };
            let line_count = text.lines().count();
            for (i, line) in text.lines().enumerate() {
                drawer.canvas().set_text(x, y, line, style);
                if i < line_count - 1 {
                    y += 1;
                    x = 0;
                } else {
                    x += line.width() as isize;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_mixed_text() {
        assert_eq!(element!(MixedText).to_string(), "\n");

        assert_eq!(
            element! {
                View(width: 14) {
                    MixedText(contents: vec![
                        MixedTextContent::new("this is ").color(Color::Red).weight(Weight::Bold),
                        MixedTextContent::new("a wrapping test").decoration(TextDecoration::Underline),
                    ])
                }
            }
            .to_string(),
            "this is a\nwrapping test\n"
        );
    }
}
