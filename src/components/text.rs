use crate::{Component, ComponentProps, ComponentRenderer, ComponentUpdater, NodeId};
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
    node_id: NodeId,
    props: TextProps,
}

impl Component for Text {
    type Props = TextProps;
    type State = ();

    fn new(node_id: NodeId, props: Self::Props) -> Self {
        Self {
            node_id,
            props: props.clone(),
        }
    }

    fn set_props(&mut self, props: Self::Props) {
        self.props = props.clone();
    }

    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn update(&mut self, mut updater: ComponentUpdater<'_>) {
        let width = self.props.value.len() as f32;
        updater.set_measure_func(Box::new(move |_, _, _| Size { width, height: 1.0 }));
    }

    fn render(&self, renderer: ComponentRenderer<'_>) {
        renderer.queue(style::Print(self.props.value.clone()));
    }
}
