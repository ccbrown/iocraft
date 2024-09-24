use std::marker::PhantomData;

/// This trait makes a struct available for use as component properties.
///
/// # Examples
///
/// ```
/// # use iocraft::prelude::*;
/// #[derive(Default, Props)]
/// struct MyProps {
///    foo: String,
/// }
/// ```
///
/// Unowned data is okay too:
///
/// ```
/// # use iocraft::prelude::*;
/// #[derive(Default, Props)]
/// struct MyProps<'a> {
///    foo: &'a str,
/// }
/// ```
///
/// However, a field that would make the struct
/// [invariant](https://doc.rust-lang.org/nomicon/subtyping.html) is not okay and will not compile:
///
/// ```compile_fail
/// # use iocraft::prelude::*;
/// # struct MyType<'a>(&'a str);
/// #[derive(Default, Props)]
/// struct MyProps<'a> {
///    foo: &'a mut MyType<'a>,
/// }
/// ```
///
/// Properties can be used by custom components like so:
///
/// ```
/// # use iocraft::prelude::*;
/// #[derive(Default, Props)]
/// struct GreetingProps<'a> {
///    name: &'a str,
/// }
///
/// #[component]
/// fn Greeting<'a>(props: &GreetingProps<'a>) -> impl Into<AnyElement<'a>> {
///    element! {
///        Text(content: format!("Hello, {}!", props.name))
///    }
/// }
/// ```
///
/// If you are building a component for use in a library, you will typically also want to mark your
/// props as `#[non_exhaustive]`.
///
/// # Safety
///
/// This requires the type to be [covariant](https://doc.rust-lang.org/nomicon/subtyping.html). If
/// implemented for a type that is not actually covariant, then the safety of the program is
/// compromised. You can use the [`#[derive(Props)]`](derive@crate::Props) macro to implement this trait safely. If the
/// type is not actually covariant, the derive macro will give you an error at compile-time.
pub unsafe trait Props {}

#[doc(hidden)]
#[derive(Clone, Copy, iocraft_macros::Props, Default)]
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
    pub(crate) fn owned<T: Props + 'a>(props: T) -> Self {
        let raw = Box::into_raw(Box::new(props));
        Self {
            raw: raw as *mut (),
            drop: Some(Box::new(DropRawImpl::<T> {
                _marker: PhantomData,
            })),
            _marker: PhantomData,
        }
    }

    pub(crate) fn borrowed<T: Props>(props: &'a mut T) -> Self {
        Self {
            raw: props as *const T as *mut (),
            drop: None,
            _marker: PhantomData,
        }
    }

    pub(crate) unsafe fn downcast_ref_unchecked<T: Props>(&self) -> &T {
        unsafe { &*(self.raw as *const T) }
    }

    pub(crate) unsafe fn downcast_mut_unchecked<T: Props>(&mut self) -> &mut T {
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
