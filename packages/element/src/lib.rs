#![cfg_attr(not(test), no_std)]

pub struct Element<K, T: ElementType> {
    pub key: K,
    pub props: T::Props,
}

pub trait ElementType {
    type Props;
}
