#![allow(dead_code)]

use iocraft::hooks::UseAsync;
use iocraft_macros::hooks;

#[hooks]
struct MyHooks {
    foo: UseAsync,
}
