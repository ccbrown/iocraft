use crate::{
    AnyElement, Component, ComponentProps, ComponentRenderer, ComponentUpdater, Components,
    ElementType,
};
use flashy_macros::with_layout_style_props;
use taffy::Rect;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BorderStyle {
    #[default]
    None,
    Single,
    Double,
    Round,
    Bold,
    SingleDouble,
    DoubleSingle,
    Classic,
}

#[with_layout_style_props]
#[derive(Clone, Default)]
pub struct BoxProps {
    pub children: Vec<AnyElement>,
    pub border_style: BorderStyle,
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

    fn update(&mut self, mut updater: ComponentUpdater<'_>) {
        let mut style: taffy::style::Style = self.props.layout_style().into();
        style.border = Rect::length(if self.props.border_style == BorderStyle::None {
            0.0
        } else {
            1.0
        });
        updater.set_layout_style(style);
        let mut updater = self.children.updater(updater);
        for e in self.props.children.iter().cloned() {
            updater.update(e);
        }
    }

    fn render(&self, renderer: ComponentRenderer<'_>) {
        self.children.render(renderer);
    }
}
