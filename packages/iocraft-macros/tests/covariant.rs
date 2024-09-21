#![allow(dead_code)]

use iocraft_macros::Covariant;

#[derive(Covariant)]
struct Unit;

#[derive(Covariant)]
struct BasicStruct {
    foo: i32,
}

#[derive(Covariant)]
struct StructWithLifetime<'lt> {
    foo: &'lt i32,
    bar: &'lt mut i32,
}

#[derive(Covariant)]
struct StructWithLifetimeAndConsts<'lt, const N: usize, const M: usize> {
    foo: &'lt i32,
    bar: &'lt mut i32,
    baz: [i32; N],
    qux: [i32; M],
}

#[derive(Covariant)]
struct StructWithTypeGeneric<T> {
    foo: T,
}

#[derive(Covariant)]
struct StructWithLifetimeAndTypeGeneric<'lt, T> {
    foo: &'lt T,
}
