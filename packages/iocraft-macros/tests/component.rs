#![allow(dead_code)]

use iocraft::{components::Box, AnyElement};
use iocraft_macros::{component, element};

#[component]
fn MyComponent() -> impl Into<AnyElement<'static>> {
    element!(Box)
}
