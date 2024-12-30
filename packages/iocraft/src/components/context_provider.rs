use crate::{AnyElement, Component, ComponentUpdater, Context, Hooks, Props};

/// The props which can be passed to the [`ContextProvider`] component.
#[non_exhaustive]
#[derive(Default, Props)]
pub struct ContextProviderProps<'a> {
    /// The children of the component.
    pub children: Vec<AnyElement<'a>>,

    /// The context to provide to the children.
    pub value: Option<Context<'a>>,
}

/// `ContextProvider` is a component that provides a context to its children.
///
/// Once a context is provided, it can be accessed by the children using the
/// [`UseContext`](crate::hooks::UseContext) hook.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// struct NumberOfTheDay(i32);
///
/// #[component]
/// fn MyContextConsumer(hooks: Hooks) -> impl Into<AnyElement<'static>> {
///     let number = hooks.use_context::<NumberOfTheDay>();
///
///     element! {
///         View(border_style: BorderStyle::Round, border_color: Color::Cyan) {
///             Text(content: "The number of the day is... ")
///             Text(color: Color::Green, weight: Weight::Bold, content: number.0.to_string())
///             Text(content: "!")
///         }
///     }
/// }
///
/// fn main() {
///     element! {
///         ContextProvider(value: Context::owned(NumberOfTheDay(42))) {
///             MyContextConsumer
///         }
///     }
///     .print();
/// }
/// ```
#[derive(Default)]
pub struct ContextProvider;

impl Component for ContextProvider {
    type Props<'a> = ContextProviderProps<'a>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        updater.set_transparent_layout(true);
        updater.update_children(
            props.children.iter_mut(),
            props.value.as_mut().map(|cx| cx.borrow()),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    struct StringContext(String);

    #[component]
    fn MyComponent(hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let s = hooks.use_context::<StringContext>();

        let _ = hooks.use_context_mut::<StringContext>();

        hooks
            .try_use_context::<StringContext>()
            .expect("context not found");

        hooks
            .try_use_context_mut::<StringContext>()
            .expect("mutable context not found");

        element! {
            Text(content: &s.0)
        }
    }

    #[test]
    fn test_context_provider() {
        let context_by_ref = StringContext("x".into());
        let mut context_by_mut_ref = StringContext("y".into());
        assert_eq!(
            element! {
                ContextProvider(value: Context::from_ref(&context_by_ref)) {
                    ContextProvider(value: Context::from_mut(&mut context_by_mut_ref)) {
                        ContextProvider(value: Context::owned(StringContext("foo".into()))) {
                            MyComponent
                        }
                    }
                }
            }
            .to_string(),
            "foo\n"
        );
    }
}
