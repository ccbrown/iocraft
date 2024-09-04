use core::any::Any;
use flashy_io::{Element, ElementKey, ElementType};
use flashy_macros::flashy;

struct MyComponent;

#[derive(Default)]
struct MyComponentProps {
    foo: String,
    children: Vec<Element<MyComponent>>,
}

impl ElementType for MyComponent {
    type Props = MyComponentProps;
}

pub struct AnyElement {
    pub key: ElementKey,
    pub props: Box<dyn Any>,
}

impl<T: ElementType> From<Element<T>> for AnyElement
where
    T::Props: 'static,
{
    fn from(e: Element<T>) -> Self {
        AnyElement {
            key: e.key,
            props: Box::new(e.props),
        }
    }
}

struct MyContainer;

#[derive(Default)]
struct MyContainerProps {
    children: Vec<AnyElement>,
}

impl ElementType for MyContainer {
    type Props = MyContainerProps;
}

#[test]
fn minimal() {
    let _: Element<MyComponent> = flashy!(MyComponent);
}

#[test]
fn props() {
    let e = flashy! {
        MyComponent(foo: "bar")
    };
    assert_eq!(e.props.foo, "bar");
}

#[test]
fn children() {
    let e = flashy! {
        MyComponent {
            MyComponent(foo: "bar")
        }
    };
    assert_eq!(e.props.children.len(), 1);
    assert_eq!(e.props.children[0].props.foo, "bar");
}

#[test]
fn any_children() {
    let mut e = flashy! {
        MyContainer {
            MyContainer
            MyComponent(foo: "bar")
        }
    };
    assert_eq!(e.props.children.len(), 2);
    assert_eq!(
        e.props
            .children
            .pop()
            .unwrap()
            .props
            .downcast::<MyComponentProps>()
            .unwrap()
            .foo,
        "bar"
    );
}
