use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    mem,
};

/// The system context, which is always available to all components.
pub struct SystemContext {
    should_exit: bool,
}

impl SystemContext {
    pub(crate) fn new() -> Self {
        Self { should_exit: false }
    }

    /// If called from a component that is being dynamically rendered, this will cause the render
    /// loop to exit and return to the caller after the current render pass.
    pub fn exit(&mut self) {
        self.should_exit = true;
    }

    pub(crate) fn should_exit(&self) -> bool {
        self.should_exit
    }
}

/// A context that can be passed to components.
pub enum Context<'a> {
    /// Provides the context via a mutable reference. Children will be able to get mutable or
    /// immutable references to the context.
    Mut(&'a mut (dyn Any + Send + Sync)),
    /// Provides the context via an immutable reference. Children will not be able to get a mutable
    /// reference to the context.
    Ref(&'a (dyn Any + Send + Sync)),
    /// Provides the context via an owned value. Children will be able to get mutable or immutable
    /// references to the context.
    Owned(Box<(dyn Any + Send + Sync)>),
}

impl<'a> Context<'a> {
    /// Creates a new context from an owned value. Children will be able to get mutable or
    /// immutable references to the context.
    pub fn owned<T: Any + Send + Sync>(context: T) -> Self {
        Context::Owned(Box::new(context))
    }

    /// Creates a new context from a mutable reference. Children will be able to get mutable or
    /// immutable references to the context.
    pub fn from_mut<T: Any + Send + Sync>(context: &'a mut T) -> Self {
        Context::Mut(context)
    }

    /// Creates a new context from an immutable reference. Children will not be able to get a
    /// mutable reference to the context.
    pub fn from_ref<T: Any + Send + Sync>(context: &'a T) -> Self {
        Context::Ref(context)
    }

    #[doc(hidden)]
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        match self {
            Context::Mut(context) => context.downcast_ref::<T>(),
            Context::Ref(context) => context.downcast_ref::<T>(),
            Context::Owned(context) => context.downcast_ref::<T>(),
        }
    }

    #[doc(hidden)]
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        match self {
            Context::Mut(context) => context.downcast_mut::<T>(),
            Context::Ref(_) => None,
            Context::Owned(context) => context.downcast_mut::<T>(),
        }
    }

    #[doc(hidden)]
    pub fn borrow(&mut self) -> Context {
        match self {
            Context::Mut(context) => Context::Mut(*context),
            Context::Ref(context) => Context::Ref(*context),
            Context::Owned(context) => Context::Mut(&mut **context),
        }
    }
}

#[doc(hidden)]
pub struct ContextStack<'a> {
    contexts: Vec<RefCell<Context<'a>>>,
}

impl<'a> ContextStack<'a> {
    pub(crate) fn root(root_context: &'a mut (dyn Any + Send + Sync)) -> Self {
        Self {
            contexts: vec![RefCell::new(Context::Mut(root_context))],
        }
    }

    pub(crate) fn with_context<'b, F>(&'b mut self, context: Option<Context<'b>>, f: F)
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
