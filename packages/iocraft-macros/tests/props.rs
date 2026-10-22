#![allow(dead_code)]

use iocraft_macros::Props;

#[derive(Props)]
struct Unit;

#[derive(Props)]
struct BasicStruct {
    foo: i32,
}

#[derive(Props)]
struct StructWithLifetime<'lt> {
    foo: &'lt i32,
    bar: &'lt mut i32,
}

#[derive(Props)]
struct StructWithLifetimeAndConsts<'lt, const N: usize, const M: usize> {
    foo: &'lt i32,
    bar: &'lt mut i32,
    baz: [i32; N],
    qux: [i32; M],
}

#[derive(Props)]
struct StructWithTypeGeneric<T: Send + Sync> {
    foo: T,
}

#[derive(Props)]
struct StructWithLifetimeAndTypeGeneric<'lt, T: Sync> {
    foo: &'lt T,
}

#[derive(Props)]
struct RecursiveProps {
    next: Option<Box<Self>>,
}

#[derive(Props)]
struct GenericRecursiveProps<'a, T: Sync> {
    #[iocraft(required)]
    value: &'a T,
    next: Option<Box<Self>>,
}

trait AssociatedValue {
    type Value: Default;
}

#[derive(Props)]
struct AssociatedSelfProps {
    value: <Self as AssociatedValue>::Value,
}

impl AssociatedValue for AssociatedSelfProps {
    type Value = u32;
}

#[derive(Props)]
struct ConstSelfProps {
    value: [u8; Self::WIDTH],
}

impl ConstSelfProps {
    const WIDTH: usize = 4;
}

#[derive(Props)]
struct SelfBoundProps<T: Into<Self> + Send + Sync> {
    value: Option<T>,
}

#[derive(Props)]
struct NestedItemSelfProps {
    value: [u8; {
        struct Local;
        impl Local {
            const WIDTH: usize = 2;
            const SIZE: usize = Self::WIDTH;
        }
        Local::SIZE
    }],
    next: Option<Box<Self>>,
}

#[derive(Props)]
struct StructWithLifetimeAndDefaultedType<'lt, T: Sync = i32> {
    foo: &'lt T,
}

#[derive(Props)]
struct StructWithLifetimeAndDefaultedConst<'lt, const N: usize = 4> {
    foo: &'lt [i32; N],
}
