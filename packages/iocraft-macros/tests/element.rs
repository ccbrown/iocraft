#![allow(dead_code)]

use iocraft::{element, AnyElement, Component, Element, Percent, Props};

#[derive(Default)]
struct MyComponent;

#[derive(Default, Props)]
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

#[derive(Default, Props)]
struct MyContainerProps {
    children: Vec<AnyElement<'static>>,
}

impl Component for MyContainer {
    type Props<'a> = MyContainerProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }
}

struct MyGenericComponent<T> {
    _marker: std::marker::PhantomData<*const T>,
}

#[derive(Default, Props)]
struct MyGenericComponentProps<T> {
    items: Vec<T>,
}

impl<T: 'static> Component for MyGenericComponent<T> {
    type Props<'a> = MyGenericComponentProps<T>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
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
fn code_interpolation_any() {
    let e = element! {
        MyContainer {
            MyContainer
            #(element!(MyContainer).into_any())
        }
    };
    assert_eq!(e.props.children.len(), 2);
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

#[test]
fn comment() {
    let e = element! {
        MyContainer {
            // This is a comment!
            MyContainer
        }
    };
    assert_eq!(e.props.children.len(), 1);
}

#[test]
fn key() {
    let e = element! {
        MyContainer(key: "foo") {
            MyContainer
        }
    };
    assert_eq!(e.props.children.len(), 1);
}

#[test]
fn generics() {
    let e = element! {
        MyGenericComponent<i32>(items: vec![0])
    };
    assert_eq!(vec![0], e.props.items);
}
