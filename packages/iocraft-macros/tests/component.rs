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

#[derive(Props)]
struct MyGenericProps<T, const U: usize> {
    foo: [T; U],
}

#[component]
fn MyComponentWithGenericProps<T: 'static, const U: usize>(
    _props: &mut MyGenericProps<T, U>,
) -> impl Into<AnyElement<'static>> {
    element!(Box)
}

fn check_component_traits<T: Sync + Send>() {}

fn check_component_traits_with_generic<T: 'static, const U: usize>() {
    check_component_traits::<MyComponentWithGenericProps<T, U>>();
}

#[component]
fn MyComponentWithGenericPropsWhereClause<T, const U: usize>(
    _props: &mut MyGenericProps<T, U>,
) -> impl Into<AnyElement<'static>>
where
    T: 'static,
{
    element!(Box)
}
