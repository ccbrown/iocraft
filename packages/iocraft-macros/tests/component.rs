#![allow(dead_code)]

use iocraft::{components::Box, AnyElement, Hooks};
use iocraft_macros::{component, element, props};

#[component]
fn MyComponent() -> impl Into<AnyElement<'static>> {
    element!(Box)
}

#[props]
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
