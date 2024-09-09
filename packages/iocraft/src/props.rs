use std::mem::transmute;

pub unsafe trait Covariant {
    type StaticSelf: 'static;
}

#[derive(Clone, Copy, iocraft_macros::Covariant, Default)]
pub struct NoProps;

pub enum AnyProps<'a> {
    Owned(Box<dyn std::any::Any>),
    Borrowed(&'a dyn std::any::Any),
}

impl AnyProps<'static> {
    pub fn owned<T: Covariant>(props: T) -> Self {
        let props = Box::into_raw(Box::new(props));
        let props = unsafe { Box::from_raw(transmute::<*mut T, *mut T::StaticSelf>(props)) };
        Self::Owned(props)
    }
}

impl<'a> AnyProps<'a> {
    pub fn borrowed<T: Covariant>(props: &'a T) -> Self {
        let props = unsafe { transmute::<&'a T, &'a T::StaticSelf>(props) };
        Self::Borrowed(props)
    }

    pub fn downcast_ref<T: Covariant>(&self) -> Option<&T> {
        unsafe {
            transmute::<Option<&T::StaticSelf>, Option<&T>>(match self {
                Self::Owned(props) => props.downcast_ref::<T::StaticSelf>(),
                Self::Borrowed(props) => props.downcast_ref::<T::StaticSelf>(),
            })
        }
    }

    pub fn borrow(&'a self) -> Self {
        match self {
            Self::Owned(props) => Self::Borrowed(props.as_ref()),
            Self::Borrowed(props) => Self::Borrowed(*props),
        }
    }
}
