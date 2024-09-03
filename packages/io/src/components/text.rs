use crate::{Component, ComponentProps, ComponentRenderer, ComponentUpdater, ElementType};
use crossterm::style;
use taffy::Size;

#[derive(Clone, Default)]
pub struct TextProps {
    pub value: String,
}

impl ComponentProps for TextProps {
    type Component = Text;
}

pub struct Text {
    props: TextProps,
}

impl ElementType for Text {
    type Props = TextProps;
}

impl Component for Text {
    type Props = TextProps;
    type State = ();

    fn new(props: Self::Props) -> Self {
        Self {
            props: props.clone(),
        }
    }

    fn set_props(&mut self, props: Self::Props) {
        self.props = props.clone();
    }

    fn update(&mut self, mut updater: ComponentUpdater<'_>) {
        let width = self.props.value.len() as f32;
        updater.set_measure_func(Box::new(move |_, _, _| Size { width, height: 1.0 }));
    }

    fn render(&self, renderer: ComponentRenderer<'_>) {
        renderer.queue(style::Print(self.props.value.clone()));
    }
}
