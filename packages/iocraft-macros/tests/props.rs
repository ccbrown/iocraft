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
struct StructWithTypeGeneric<T> {
    foo: T,
}

#[derive(Props)]
struct StructWithLifetimeAndTypeGeneric<'lt, T> {
    foo: &'lt T,
}
