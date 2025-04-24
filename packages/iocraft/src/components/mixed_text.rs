use crate::{
    components::text::{Text, TextAlign, TextDecoration, TextDrawer, TextWrap},
    segmented_string::SegmentedString,
    CanvasTextStyle, Color, Component, ComponentDrawer, ComponentUpdater, Hooks, Props, Weight,
};

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

    /// Whether to italicize the text.
    pub italic: bool,
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

    /// Returns a new [`MixedTextContent`] with italic text.
    pub fn italic(mut self) -> Self {
        self.italic = true;
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
        let plaintext = props
            .contents
            .iter()
            .map(|content| content.text.as_str())
            .collect::<Vec<_>>()
            .join("");
        self.contents = props.contents.clone();
        self.wrap = props.wrap;
        self.align = props.align;
        updater.set_measure_func(Text::measure_func(plaintext, props.wrap));
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_>) {
        let width = drawer.layout().size.width;
        let segmented_string: SegmentedString = self
            .contents
            .iter()
            .map(|content| content.text.as_str())
            .collect();
        let lines = segmented_string.wrap(match self.wrap {
            TextWrap::Wrap => width as usize,
            TextWrap::NoWrap => usize::MAX,
        });
        let mut drawer = TextDrawer::new(drawer, self.align != TextAlign::Left);
        for mut line in lines {
            if self.wrap == TextWrap::Wrap {
                line.trim_end();
            }
            let padding = Text::alignment_padding(line.width, self.align, width as _);
            if padding > 0 {
                drawer.append_lines(
                    [format!("{:width$}", "", width = padding).as_str()],
                    CanvasTextStyle::default(),
                );
            }
            let mut segments = line.segments.into_iter().peekable();
            while let Some(segment) = segments.next() {
                let content = &self.contents[segment.index];
                let style = CanvasTextStyle {
                    color: content.color,
                    weight: content.weight,
                    underline: content.decoration == TextDecoration::Underline,
                    italic: content.italic,
                };
                if segments.peek().is_some() {
                    drawer.append_lines([segment.text], style);
                } else {
                    drawer.append_lines([segment.text, ""], style);
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
                        MixedTextContent::new("this is ").color(Color::Red).weight(Weight::Bold).italic(),
                        MixedTextContent::new("a wrapping test").decoration(TextDecoration::Underline),
                    ])
                }
            }
            .to_string(),
            "this is a\nwrapping test\n"
        );
    }
}
