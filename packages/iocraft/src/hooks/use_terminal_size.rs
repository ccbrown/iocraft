use crate::{
    hooks::{UseState, UseTerminalEvents},
    Hooks, TerminalEvent,
};
use crossterm::terminal;

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// `UseTerminalSize` is a hook that returns the current terminal size.
pub trait UseTerminalSize: private::Sealed {
    /// Returns the current terminal size as a tuple of `(width, height)`.
    fn use_terminal_size(&mut self) -> (u16, u16);
}

impl UseTerminalSize for Hooks<'_, '_> {
    fn use_terminal_size(&mut self) -> (u16, u16) {
        let size = self.use_state(|| terminal::size().unwrap_or((0, 0)));
        self.use_terminal_events(move |event| {
            if let TerminalEvent::Resize(width, height) = event {
                size.set((width, height));
            }
        });
        size.get()
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use futures::stream::StreamExt;
    use macro_rules_attribute::apply;
    use smol_macros::test;

    #[component]
    fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let (width, height) = hooks.use_terminal_size();

        if width == 100 && height == 40 {
            system.exit();
        }

        element! {
            Text(content: format!("{}x{}", width, height))
        }
    }

    #[apply(test!)]
    async fn test_use_terminal_size() {
        let actual = element!(MyComponent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(futures::stream::iter(
                vec![TerminalEvent::Resize(100, 40)],
            )))
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .await;
        assert_eq!(actual.last().unwrap(), "100x40\n");
    }
}
