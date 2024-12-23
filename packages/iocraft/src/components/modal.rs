use crate::{AnyElement, Component};
use iocraft_macros::{with_layout_style_props, Props};

/// Defines properties for a `Modal`.
#[non_exhaustive]
#[with_layout_style_props]
#[derive(Default, Props)]
pub struct ModalProps<'a> {
    /// The elements to render inside of the `Modal`.
    pub children: Vec<AnyElement<'a>>,

    /// Whether to render the `Modal`.
    pub is_open: bool,
    // TODO(nyanzebra): after open and after close events?
}

/// `Modal` is a component that renders a 'popup'.
#[derive(Default)]
pub struct Modal {
    /// Whether to render the `Modal`.
    is_open: bool,
    // TODO(nyanzebra): after open and after close events?
}

impl Component for Modal {
    type Props<'a> = ModalProps<'a>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self::default()
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: crate::Hooks,
        updater: &mut crate::ComponentUpdater,
    ) {
        updater.set_transparent_layout(true);
        self.is_open = props.is_open;
        let mut style: taffy::style::Style = props.layout_style().into();
        if self.is_open {
            style.display = taffy::Display::Flex;
            updater.update_children(props.children.iter_mut(), None);
        } else {
            let mut empty: Vec<AnyElement<'_>> = vec![];
            updater.update_children(empty.iter_mut(), None);
            style.display = taffy::Display::None;
        }
        updater.set_layout_style(style);
    }

    fn draw(&mut self, _drawer: &mut crate::ComponentDrawer<'_>) {}
}
