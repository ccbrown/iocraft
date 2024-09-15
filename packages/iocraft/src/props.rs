use std::marker::PhantomData;

/// This trait marks a type as being covariant.
///
/// # Safety
///
/// If the type is not actually covariant, then the safety of the program is compromised. You can
/// use the `#[derive(Covariant)]` macro to implement this trait safely. If the type is not
/// actually covariant, the derive macro will not compile.
pub unsafe trait Covariant {}

#[doc(hidden)]
#[derive(Clone, Copy, iocraft_macros::Covariant, Default)]
pub struct NoProps;

struct DropRawImpl<T> {
    _marker: PhantomData<T>,
}

trait DropRaw {
    fn drop_raw(&self, raw: *mut ());
}

impl<T> DropRaw for DropRawImpl<T> {
    fn drop_raw(&self, raw: *mut ()) {
        unsafe {
            let _ = Box::from_raw(raw as *mut T);
        }
    }
}

#[doc(hidden)]
pub struct AnyProps<'a> {
    raw: *mut (), // *T
    drop: Option<Box<dyn DropRaw + 'a>>,
    _marker: PhantomData<&'a mut ()>,
}

impl<'a> AnyProps<'a> {
    pub(crate) fn owned<T: Covariant + 'a>(props: T) -> Self {
        let raw = Box::into_raw(Box::new(props));
        Self {
            raw: raw as *mut (),
            drop: Some(Box::new(DropRawImpl::<T> {
                _marker: PhantomData,
            })),
            _marker: PhantomData,
        }
    }

    pub(crate) fn borrowed<T: Covariant>(props: &'a mut T) -> Self {
        Self {
            raw: props as *const T as *mut (),
            drop: None,
            _marker: PhantomData,
        }
    }

    pub(crate) unsafe fn downcast_ref_unchecked<T: Covariant>(&self) -> &T {
        unsafe { &*(self.raw as *const T) }
    }

    pub(crate) unsafe fn downcast_mut_unchecked<T: Covariant>(&mut self) -> &mut T {
        unsafe { &mut *(self.raw as *mut T) }
    }

    pub(crate) fn borrow(&mut self) -> Self {
        Self {
            raw: self.raw,
            drop: None,
            _marker: PhantomData,
        }
    }
}

impl Drop for AnyProps<'_> {
    fn drop(&mut self) {
        if let Some(drop) = self.drop.take() {
            drop.drop_raw(self.raw);
        }
    }
}
