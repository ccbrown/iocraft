use crate::{AnyElement, Component, ComponentUpdater, Hooks, Props};

/// The props which can be passed to the [`Fragment`] component.
#[non_exhaustive]
#[derive(Default, Props)]
pub struct FragmentProps<'a> {
    /// The children of the component.
    pub children: Vec<AnyElement<'a>>,
}

/// `Fragment` is a component which allows you to group elements without impacting the resulting
/// layout.
///
/// This is typically used when you want to create a component that returns multiple elements.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// #[component]
/// fn TextLines() -> impl Into<AnyElement<'static>> {
///     element! {
///         Fragment {
///             Text(content: "Line 1")
///             Text(content: "Line 2")
///         }
///     }
/// }
///
/// fn MyComponent() -> impl Into<AnyElement<'static>> {
///     element! {
///         View(flex_direction: FlexDirection::Column) {
///             TextLines
///         }
///     }
/// }
/// ```
#[derive(Default)]
pub struct Fragment;

impl Component for Fragment {
    type Props<'a> = FragmentProps<'a>;

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
        updater.update_children(props.children.iter_mut(), None);
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[component]
    fn TextLines() -> impl Into<AnyElement<'static>> {
        element! {
            Fragment {
                Text(content: "Line 1")
                Text(content: "Line 2")
            }
        }
    }

    #[component]
    fn MyComponent() -> impl Into<AnyElement<'static>> {
        element! {
            View(flex_direction: FlexDirection::Column) {
                TextLines
            }
        }
    }

    #[test]
    fn test_fragment() {
        assert_eq!(element!(MyComponent).to_string(), "Line 1\nLine 2\n");
    }
}
