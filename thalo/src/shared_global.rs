use once_cell::sync::Lazy;
use std::{any::Any, collections::hash_map::RandomState, hash::Hash, marker::PhantomData};
use typedmap::{dashmap::Ref, TypedDashMap, TypedMapKey};

static MAPPING: Lazy<TypedDashMap> = Lazy::new(TypedDashMap::new);

pub struct Key<T, V>
where
    T: 'static,
    V: 'static,
{
    _phantomt: PhantomData<T>,
    _phantomv: PhantomData<V>,
}

impl<T, V> PartialEq for Key<T, V> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<T, V> Eq for Key<T, V> {}

impl<T, V> Hash for Key<T, V>
where
    T: 'static,
    V: 'static,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Any::type_id(self).hash(state)
    }
}

impl<T, V> TypedMapKey for Key<T, V> {
    type Value = V;
}

pub trait SharedGlobal<Act, ES>: Sized + Send + Sync + 'static {
    type Value: Send + Sync + 'static;

    fn set(t: Self::Value) {
        let key = Key {
            _phantomt: PhantomData,
            _phantomv: PhantomData,
        };
        MAPPING.insert::<Key<Self, Self::Value>>(key, t);
    }

    fn get() -> Option<Ref<'static, (), Key<Self, Self::Value>, RandomState>> {
        let key = Key {
            _phantomt: PhantomData::<Self>,
            _phantomv: PhantomData::<Self::Value>,
        };
        MAPPING.get(&key)
    }

    fn remove() {
        let key = Key {
            _phantomt: PhantomData::<Self>,
            _phantomv: PhantomData::<Self::Value>,
        };
        MAPPING.remove(&key);
    }
}
