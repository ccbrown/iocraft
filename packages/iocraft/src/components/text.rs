use crate::{Color, Component, ComponentRenderer, ComponentUpdater, TextStyle, Weight};
use taffy::Size;

#[derive(Clone, Default)]
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
    type Props = TextProps;

    fn new(_props: &Self::Props) -> Self {
        Self::default()
    }

    fn update(&mut self, props: &Self::Props, updater: &mut ComponentUpdater<'_>) {
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
        assert_eq!(element!(Text).to_string(), "\n");

        assert_eq!(element!(Text(content: "foo")).to_string(), "foo\n");
    }
}
