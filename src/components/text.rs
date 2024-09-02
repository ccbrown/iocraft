use crate::{Component, NodeId, Renderable, TreeRenderer, TreeUpdater};
use crossterm::style;
use taffy::Size;

pub struct TextProps {
    pub value: String,
}

pub struct Text {
    node_id: NodeId,
    props: TextProps,
}

impl Renderable for Text {
    fn update(&mut self, mut updater: TreeUpdater<'_>) {
        let width = self.props.value.len() as f32;
        updater.set_measure_func(Box::new(move |_, _, _| Size { width, height: 1.0 }));
    }

    fn render(&self, renderer: TreeRenderer<'_>) {
        renderer.queue(style::Print(self.props.value.clone()));
    }
}

impl Component for Text {
    type Props = TextProps;
    type State = ();

    fn new(node_id: NodeId, props: Self::Props) -> Self {
        Self { node_id, props }
    }

    fn set_props(&mut self, props: Self::Props) {
        self.props = props;
    }

    fn node_id(&self) -> NodeId {
        self.node_id
    }
}
