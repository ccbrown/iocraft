use taffy::Rect;

use crate::{
    hooks::{Ref, UseRef},
    ComponentDrawer, Hook, Hooks,
};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// `UseComponentRect` is a hook that returns the current component's canvas position and size
///  from the previous frame.
///
/// See [`ComponentDrawer::canvas_position`] and [`ComponentDrawer::size`] for more info.
///
/// <div class="warning">For the first time rendering, it will return exactly (0, 0, 0, 0) rectangle !</div>
pub trait UseComponentRect<'a>: private::Sealed {
    /// Returns the curent component canvas position and size in form of a [`Rect`].
    fn use_component_rect(&mut self) -> Ref<Rect<u16>>;
}

impl<'a> UseComponentRect<'a> for Hooks<'a, '_> {
    fn use_component_rect(&mut self) -> Ref<Rect<u16>> {
        let rect = self.use_ref_default();
        self.use_hook(|| UseComponentRectImpl { rect }).rect
    }
}

struct UseComponentRectImpl {
    rect: Ref<Rect<u16>>,
}

impl Hook for UseComponentRectImpl {
    fn pre_component_draw(&mut self, drawer: &mut ComponentDrawer) {
        let size = drawer.size();
        let position = drawer.canvas_position();
        self.rect.set(Rect {
            left: position.x as u16,
            right: position.x as u16 + size.width,
            top: position.y as u16,
            bottom: position.y as u16 + size.height,
        });
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
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut frame = hooks.use_state(|| 0);
        let rect = hooks.use_component_rect().get();

        // Notice that we have to wait one frame for the correct size and position.
        if frame.get() >= 1 {
            system.exit();
        }
        frame += 1;

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
        .mock_terminal_render_loop(MockTerminalConfig::default())
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .await;
        assert_eq!(actual.last().unwrap().trim(), "17:24:25:26");
    }
}
