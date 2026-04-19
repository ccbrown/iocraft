use std::{
    any::Any,
    fmt,
    hash::{Hash, Hasher},
};

pub(crate) trait AnyKey: Any {
    fn as_any(&self) -> &dyn Any;
    fn dyn_eq(&self, other: &dyn AnyKey) -> bool;
    fn dyn_hash(&self, state: &mut dyn Hasher);
}

impl<T: Any + Eq + Hash> AnyKey for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &dyn AnyKey) -> bool {
        other.as_any().downcast_ref::<T>() == Some(self)
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }
}

impl fmt::Debug for dyn AnyKey + Send + Sync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_any().fmt(f)
    }
}

impl PartialEq for dyn AnyKey + Send + Sync {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other)
    }
}

impl Eq for dyn AnyKey + Send + Sync {}

impl Hash for dyn AnyKey + Send + Sync {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state)
    }
}
