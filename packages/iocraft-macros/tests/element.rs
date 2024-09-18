#![allow(dead_code)]

use iocraft::{element, AnyElement, Component, Covariant, Element, Percent};

#[derive(Default)]
struct MyComponent;

#[derive(Covariant, Default)]
struct MyComponentProps {
    foo: String,
    percent: Percent,
    children: Vec<Element<'static, MyComponent>>,
}

impl Component for MyComponent {
    type Props<'a> = MyComponentProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }
}

struct MyContainer;

#[derive(Covariant, Default)]
struct MyContainerProps {
    children: Vec<AnyElement<'static>>,
}

impl Component for MyContainer {
    type Props<'a> = MyContainerProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }
}

#[test]
fn minimal() {
    let _: Element<MyComponent> = element!(MyComponent);
}

#[test]
fn fully_qualified_type() {
    pub mod foo {
        pub mod bar {
            pub type MyComponent = crate::MyComponent;
        }
    }
    let _: Element<MyComponent> = element!(foo::bar::MyComponent);
    let _: Element<::iocraft::components::Box> = element!(::iocraft::components::Box);
}

#[test]
fn props() {
    let e = element! {
        MyComponent(foo: "bar")
    };
    assert_eq!(e.props.foo, "bar");
}

#[test]
fn children() {
    let e = element! {
        MyComponent {
            MyComponent(foo: "bar")
        }
    };
    assert_eq!(e.props.children.len(), 1);
    assert_eq!(e.props.children[0].props.foo, "bar");
}

#[test]
fn any_children() {
    let e = element! {
        MyContainer {
            MyContainer
            MyComponent(foo: "bar")
        }
    };
    assert_eq!(e.props.children.len(), 2);
}

#[test]
fn code_interpolation_none() {
    let e = element! {
        MyContainer {
            MyContainer
            #(None::<AnyElement<'static>>)
        }
    };
    assert_eq!(e.props.children.len(), 1);
}

#[test]
fn code_interpolation_single_child() {
    let e = element! {
        MyContainer {
            MyContainer
            #(element!(MyComponent))
        }
    };
    assert_eq!(e.props.children.len(), 2);
}

#[test]
fn percent() {
    let e = element! {
        MyContainer {
            MyComponent(percent: 50pct)
            MyComponent(percent: 50.0pct)
        }
    };
    assert_eq!(e.props.children.len(), 2);
}
