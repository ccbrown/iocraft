use crate::{Color, Component, ComponentRenderer, ComponentUpdater, Covariant, TextStyle, Weight};
use taffy::Size;

#[derive(Default, Covariant)]
pub struct TextProps {
    pub color: Option<Color>,
    pub content: String,
    pub weight: Weight,
}

#[derive(Default)]
pub struct Text {
    style: TextStyle,
    content: String,
}

impl Component for Text {
    type Props<'a> = TextProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self::default()
    }

    fn update(&mut self, props: &mut Self::Props<'_>, updater: &mut ComponentUpdater) {
        self.style = TextStyle {
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
        assert_eq!(element!(Text).into_string(), "\n");

        assert_eq!(element!(Text(content: "foo")).into_string(), "foo\n");
    }
}
