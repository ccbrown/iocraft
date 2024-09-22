#![allow(dead_code)]

use iocraft::{components::Box, AnyElement, Hooks};
use iocraft_macros::{component, element, Props};

#[component]
fn MyComponent() -> impl Into<AnyElement<'static>> {
    element!(Box)
}

#[derive(Default, Props)]
struct MyProps {
    foo: String,
}

#[component]
fn MyComponentWithProps(_props: &mut MyProps) -> impl Into<AnyElement<'static>> {
    element!(Box)
}

#[component]
fn MyComponentWithHooks(_hooks: Hooks) -> impl Into<AnyElement<'static>> {
    element!(Box)
}

#[component]
fn MyComponentWithHooksRef(_hooks: &mut Hooks) -> impl Into<AnyElement<'static>> {
    element!(Box)
}
