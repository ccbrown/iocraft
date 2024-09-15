use crate::ComponentUpdater;
use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    mem,
    ops::{Deref, DerefMut},
};

pub struct SystemContext {
    should_exit: bool,
}

impl SystemContext {
    pub(crate) fn new() -> Self {
        Self { should_exit: false }
    }

    pub fn exit(&mut self) {
        self.should_exit = true;
    }

    pub(crate) fn should_exit(&self) -> bool {
        self.should_exit
    }
}

pub enum Context<'a> {
    Mut(&'a mut dyn Any),
    Ref(&'a dyn Any),
    Owned(Box<dyn Any>),
}

impl<'a> Context<'a> {
    pub fn owned<T: Any>(context: T) -> Self {
        Context::Owned(Box::new(context))
    }

    pub fn from_mut<T: Any>(context: &'a mut T) -> Self {
        Context::Mut(context)
    }

    pub fn from_ref<T: Any>(context: &'a T) -> Self {
        Context::Ref(context)
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        match self {
            Context::Mut(context) => context.downcast_ref::<T>(),
            Context::Ref(context) => context.downcast_ref::<T>(),
            Context::Owned(context) => context.downcast_ref::<T>(),
        }
    }

    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        match self {
            Context::Mut(context) => context.downcast_mut::<T>(),
            Context::Ref(_) => None,
            Context::Owned(context) => context.downcast_mut::<T>(),
        }
    }

    pub fn borrow(&mut self) -> Context {
        match self {
            Context::Mut(context) => Context::Mut(*context),
            Context::Ref(context) => Context::Ref(*context),
            Context::Owned(context) => Context::Mut(&mut **context),
        }
    }
}

pub(crate) struct ContextStack<'a> {
    contexts: Vec<RefCell<Context<'a>>>,
}

impl<'a> ContextStack<'a> {
    pub fn root(root_context: &'a mut dyn Any) -> Self {
        Self {
            contexts: vec![RefCell::new(Context::Mut(root_context))],
        }
    }

    pub fn with_context<'b, F>(&'b mut self, context: Option<Context<'b>>, f: F)
    where
        F: FnOnce(&mut ContextStack),
    {
        if let Some(context) = context {
            // SAFETY: Mutable references to this struct are invariant over 'a, so in order to
            // append a shorter-lived context, we need to transmute 'a to the shorter lifetime.
            //
            // This is only safe because we don't allow any other changes to the stack, and we
            // revert the stack right after the call.
            let shorter_lived_self =
                unsafe { mem::transmute::<&mut Self, &mut ContextStack<'b>>(self) };
            shorter_lived_self.contexts.push(RefCell::new(context));
            f(shorter_lived_self);
            shorter_lived_self.contexts.pop();
        } else {
            f(self);
        }
    }

    pub fn get_context<T: Any>(&self) -> Option<Ref<T>> {
        for context in self.contexts.iter().rev() {
            if let Ok(context) = context.try_borrow() {
                if let Ok(ret) = Ref::filter_map(context, |context| context.downcast_ref::<T>()) {
                    return Some(ret);
                }
            }
        }
        None
    }

    pub fn get_context_mut<T: Any>(&self) -> Option<RefMut<T>> {
        for context in self.contexts.iter().rev() {
            if let Ok(context) = context.try_borrow_mut() {
                if let Ok(ret) = RefMut::filter_map(context, |context| context.downcast_mut::<T>())
                {
                    return Some(ret);
                }
            }
        }
        None
    }
}

#[doc(hidden)]
pub trait ContextImplExt<'a> {
    type Refs<'b: 'a>;

    fn refs_from_component_updater<'b: 'a>(updater: &'b ComponentUpdater) -> Self::Refs<'b>;

    fn borrow_refs<'b: 'a, 'c: 'b>(refs: &'b mut Self::Refs<'c>) -> Self;
}

#[doc(hidden)]
pub trait ContextRef<'a> {
    type Ref;
    type RefOwner<'r>;

    fn get_from_component_updater(updater: &'a ComponentUpdater) -> Self::RefOwner<'a>;
    fn borrow<'r: 'a>(owner: &'a mut Self::RefOwner<'r>) -> Self::Ref;
}

impl<'a, T: Any> ContextRef<'a> for &'a T {
    type Ref = &'a T;
    type RefOwner<'r> = Ref<'r, T>;

    fn get_from_component_updater(updater: &'a ComponentUpdater) -> Self::RefOwner<'a> {
        updater.get_context::<T>().unwrap()
    }

    fn borrow<'r: 'a>(owner: &'a mut Self::RefOwner<'r>) -> Self::Ref {
        &*owner
    }
}

impl<'a, T: Any> ContextRef<'a> for &'a mut T {
    type Ref = &'a mut T;
    type RefOwner<'r> = RefMut<'r, T>;

    fn get_from_component_updater(updater: &'a ComponentUpdater) -> Self::RefOwner<'a> {
        updater.get_context_mut::<T>().unwrap()
    }

    fn borrow<'r: 'a>(owner: &'a mut Self::RefOwner<'r>) -> Self::Ref {
        &mut *owner
    }
}

impl<'a, T: Any> ContextRef<'a> for Option<&'a T> {
    type Ref = Option<&'a T>;
    type RefOwner<'r> = Option<Ref<'r, T>>;

    fn get_from_component_updater(updater: &'a ComponentUpdater) -> Self::RefOwner<'a> {
        updater.get_context::<T>()
    }

    fn borrow<'r: 'a>(owner: &'a mut Self::RefOwner<'r>) -> Self::Ref {
        owner.as_ref().map(|r| r.deref())
    }
}

impl<'a, T: Any> ContextRef<'a> for Option<&'a mut T> {
    type Ref = Option<&'a mut T>;
    type RefOwner<'r> = Option<RefMut<'r, T>>;

    fn get_from_component_updater(updater: &'a ComponentUpdater) -> Self::RefOwner<'a> {
        updater.get_context_mut::<T>()
    }

    fn borrow<'r: 'a>(owner: &'a mut Self::RefOwner<'r>) -> Self::Ref {
        owner.as_mut().map(|r| r.deref_mut())
    }
}
