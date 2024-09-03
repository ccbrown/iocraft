use crate::{
    AnyElement, Component, ComponentProps, ComponentRenderer, ComponentUpdater, Components,
    ElementType,
};

#[derive(Clone, Default)]
pub struct BoxProps {
    pub children: Vec<AnyElement>,
}

impl ComponentProps for BoxProps {
    type Component = Box;
}

pub struct Box {
    children: Components,
    props: BoxProps,
}

impl ElementType for Box {
    type Props = BoxProps;
}

impl Component for Box {
    type Props = BoxProps;
    type State = ();

    fn new(props: Self::Props) -> Self {
        Self {
            children: Components::default(),
            props,
        }
    }

    fn set_props(&mut self, props: Self::Props) {
        self.props = props;
    }

    fn update(&mut self, updater: ComponentUpdater<'_>) {
        let mut updater = self.children.updater(updater);
        for e in self.props.children.iter().cloned() {
            updater.update(e);
        }
    }

    fn render(&self, renderer: ComponentRenderer<'_>) {
        self.children.render(renderer);
    }
}
