#![allow(dead_code)]

use iocraft::Signal;
use iocraft_macros::state;

#[state]
struct MyState {
    foo: Signal<String>,
}
