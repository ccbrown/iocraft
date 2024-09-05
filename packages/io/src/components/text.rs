use crate::{Component, ComponentProps, ComponentRenderer, ComponentUpdater, ElementType};
use crossterm::style::{Color, ContentStyle, PrintStyledContent, StyledContent};
use taffy::Size;

#[derive(Clone, Default)]
pub struct TextProps {
    pub color: Option<Color>,
    pub content: String,
}

impl ComponentProps for TextProps {
    type Component = Text;
}

pub struct Text {
    style: ContentStyle,
    content: String,
}

impl ElementType for Text {
    type Props = TextProps;
}

impl Component for Text {
    type Props = TextProps;
    type State = ();

    fn new(props: Self::Props) -> Self {
        let mut ret = Self {
            style: ContentStyle::new(),
            content: "".to_string(),
        };
        ret.set_props(props);
        ret
    }

    fn set_props(&mut self, props: Self::Props) {
        self.style.foreground_color = props.color;
        self.content = props.content;
    }

    fn update(&mut self, mut updater: ComponentUpdater<'_>) {
        let width = self.content.len() as f32;
        updater.set_measure_func(Box::new(move |_, _, _| Size { width, height: 1.0 }));
    }

    fn render(&self, renderer: &mut ComponentRenderer<'_>) {
        renderer.queue(PrintStyledContent(StyledContent::new(
            self.style,
            &self.content,
        )));
    }
}
