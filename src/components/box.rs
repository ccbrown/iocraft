use crate::{
    Component, ComponentDeclaration, ComponentProps, ComponentRenderer, ComponentUpdater,
    Components, NodeId,
};

#[derive(Clone, Default)]
pub struct BoxProps {
    pub children: Vec<ComponentDeclaration>,
}

impl ComponentProps for BoxProps {
    type Component = Box;
}

pub struct Box {
    node_id: NodeId,
    children: Components,
    props: BoxProps,
}

impl Component for Box {
    type Props = BoxProps;
    type State = ();

    fn new(node_id: NodeId, props: Self::Props) -> Self {
        Self {
            node_id,
            children: Components::default(),
            props,
        }
    }

    fn set_props(&mut self, props: Self::Props) {
        self.props = props;
    }

    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn update(&mut self, updater: ComponentUpdater<'_>) {
        let mut updater = self.children.updater(updater);
        for decl in self.props.children.iter().cloned() {
            updater.update(decl);
        }
    }

    fn render(&self, renderer: ComponentRenderer<'_>) {
        self.children.render(renderer);
    }
}
