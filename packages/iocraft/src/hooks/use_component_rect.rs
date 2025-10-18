use std::{
    pin::Pin,
    task::{Context, Poll},
};
use taffy::Rect;

use crate::{
    hooks::{Ref, UseRef},
    ComponentDrawer, Hook, Hooks,
};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// [`Ref`] with component's drawer rect.
pub type ComponentRectRef = Ref<Option<Rect<u16>>>;

/// `UseComponentRect` is a hook that returns the current component's canvas position and size
///  from the previous frame, or `None` if it's the first frame.
///
/// See [`ComponentDrawer::canvas_position`] and [`ComponentDrawer::size`] for more info.
pub trait UseComponentRect<'a>: private::Sealed {
    /// Returns the curent component canvas position and size in form of a [`Rect`].
    fn use_component_rect(&mut self) -> ComponentRectRef;
}

impl<'a> UseComponentRect<'a> for Hooks<'a, '_> {
    fn use_component_rect(&mut self) -> ComponentRectRef {
        let rect = self.use_ref_default();
        self.use_hook(move || UseComponentRectImpl {
            rect,
            is_changed: false,
        })
        .rect
    }
}

struct UseComponentRectImpl {
    rect: ComponentRectRef,
    is_changed: bool,
}

impl Hook for UseComponentRectImpl {
    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.is_changed {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    fn pre_component_draw(&mut self, drawer: &mut ComponentDrawer) {
        let size = drawer.size();
        let position = drawer.canvas_position();
        let rect = Rect {
            left: position.x as u16,
            right: position.x as u16 + size.width,
            top: position.y as u16,
            bottom: position.y as u16 + size.height,
        };

        if self.rect.get() != Some(rect) {
            self.rect.set(Some(rect));
            self.is_changed = true;
        } else if self.rect.get().is_some() {
            self.is_changed = false;
        }
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
        let rect = hooks.use_component_rect().get();

        let Some(rect) = rect else {
            return element! { Text(content: "00:00:00:00") };
        };

        system.exit();

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
        assert_eq!(actual.last().unwrap().trim(), "15:26:25:26");
    }
}
