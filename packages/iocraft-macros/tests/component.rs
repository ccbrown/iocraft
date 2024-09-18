#![allow(dead_code)]

use iocraft::{components::Box, AnyElement, Signal};
use iocraft_macros::{component, element, state};

#[component]
fn MyComponent() -> impl Into<AnyElement<'static>> {
    element!(Box)
}

#[state]
struct MyState {
    foo: Signal<u32>,
}

#[component]
fn MyComponentWithState(_state: &MyState) -> impl Into<AnyElement<'static>> {
    element!(Box)
}

#[component]
fn MyComponentWithMutState(_state: &mut MyState) -> impl Into<AnyElement<'static>> {
    element!(Box)
}
