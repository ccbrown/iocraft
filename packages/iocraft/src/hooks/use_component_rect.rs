use taffy::Rect;

use crate::{ComponentDrawer, Hook, Hooks};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// `UseComponentRect` is a hook that returns the current component canvas position and size.
///
/// See [`ComponentDrawer::canvas_position`] and [`ComponentDrawer::size`] for more info.
pub trait UseComponentRect<'a>: private::Sealed {
    /// Returns the curent component canvas position and size in form of a [`Rect`].
    fn use_component_rect(&mut self) -> Rect<u16>;
}

impl<'a> UseComponentRect<'a> for Hooks<'a, '_> {
    fn use_component_rect(&mut self) -> Rect<u16> {
        self.use_hook(UseComponentRectImpl::default).rect
    }
}

#[derive(Default)]
struct UseComponentRectImpl {
    rect: Rect<u16>,
}

impl Hook for UseComponentRectImpl {
    fn pre_component_draw(&mut self, drawer: &mut ComponentDrawer) {
        let size = drawer.size();
        let position = drawer.canvas_position();
        self.rect = Rect {
            left: position.x as u16,
            right: position.x as u16 + size.width,
            top: position.y as u16,
            bottom: position.y as u16 + size.height,
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::{hooks::use_component_rect::UseComponentRect, prelude::*};
    use futures::stream::StreamExt;
    use macro_rules_attribute::apply;
    use smol_macros::test;

    #[component]
    fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut should_exit = hooks.use_state(|| false);
        let mut system = hooks.use_context_mut::<SystemContext>();
        let rect = hooks.use_component_rect();

        hooks.use_terminal_events(move |event| match event {
            TerminalEvent::Resize(..) => should_exit.set(true),
            _ => {}
        });

        if should_exit.get() {
            system.exit();
        }

        element! {
            Text(content: format!("{}:{}:{}:{}", rect.left, rect.right, rect.top, rect.bottom))

        }
    }

    #[apply(test!)]
    async fn test_use_component_rect() {
        let actual = element!(
            View(
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                width: 40,
                height: 50,
            ) { MyComponent }
        )
        .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
            vec![TerminalEvent::Resize(40, 50)],
        )))
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .await;
        assert_eq!(actual.last().unwrap().trim(), "17:24:25:26");
    }
}
