use crate::{
    CanvasTextStyle, Color, Component, ComponentRenderer, ComponentUpdater, Covariant, Weight,
};
use taffy::Size;

/// The props which can be passed to the [`Text`] component.
#[derive(Default, Covariant)]
pub struct TextProps {
    /// The color to make the text.
    pub color: Option<Color>,

    /// The content of the text.
    pub content: String,

    /// The weight of the text.
    pub weight: Weight,
}

/// `Text` is a component that renders a text string.
#[derive(Default)]
pub struct Text {
    style: CanvasTextStyle,
    content: String,
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
        let width = self.content.len() as f32;
        updater.set_measure_func(Box::new(move |_, _, _| Size { width, height: 1.0 }));
    }

    fn render(&self, renderer: &mut ComponentRenderer<'_>) {
        renderer.canvas().set_text(0, 0, &self.content, self.style);
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_text() {
        assert_eq!(element!(Text).to_string(), "\n");

        assert_eq!(element!(Text(content: "foo")).to_string(), "foo\n");
    }
}
