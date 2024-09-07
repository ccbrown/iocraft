use crate::{Color, Component, ComponentRenderer, ComponentUpdater};
use taffy::Size;

#[derive(Clone, Default)]
pub struct TextProps {
    pub color: Option<Color>,
    pub content: String,
}

#[derive(Default)]
pub struct Text {
    color: Option<Color>,
    content: String,
}

impl Component for Text {
    type Props = TextProps;

    fn new(_props: &Self::Props) -> Self {
        Self::default()
    }

    fn update(&mut self, props: &Self::Props, updater: &mut ComponentUpdater<'_>) {
        self.color = props.color;
        self.content = props.content.clone();
        let width = self.content.len() as f32;
        updater.set_measure_func(Box::new(move |_, _, _| Size { width, height: 1.0 }));
    }

    fn render(&self, renderer: &mut ComponentRenderer<'_>) {
        renderer.canvas().set_text(0, 0, &self.content, self.color);
    }
}
