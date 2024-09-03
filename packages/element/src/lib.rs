#![cfg_attr(not(test), no_std)]
extern crate alloc;

use alloc::string::String;

pub struct Element<T: ElementType> {
    pub key: String,
    pub props: T::Props,
}

pub trait ElementType {
    type Props;
}
