//! Dicts and sets: insertion-ordered hash tables over hashable values.
//!
//! A set is a table whose values are all `None`. Entries are only ever
//! added or replaced (nothing removes one yet), so an entry's position
//! never changes and iteration reads by index.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::rt_err;
use super::value::Value;
use crate::diag::Result;
use crate::span::Span;

/// A shared, mutable table. Python dicts and sets alias, as lists do.
pub(super) type TableRef<'m> = Rc<RefCell<Table<'m>>>;

/// What a dict key or set element hashes and compares as: the hashable
/// values (grammar.md, Deviations). A `bool` is its `int`, as in Python.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum Key {
    None,
    Int(i64),
    Str(Rc<str>),
    Tuple(Vec<Key>),
}

impl Key {
    /// The key of `v`, or a `TypeError` fault for an unhashable value.
    pub fn of(v: &Value, span: Span) -> Result<Key> {
        Ok(match v {
            Value::None => Key::None,
            Value::Bool(b) => Key::Int(i64::from(*b)),
            Value::Int(i) => Key::Int(*i),
            Value::Str(s) => Key::Str(Rc::clone(s)),
            Value::Tuple(items) => Key::Tuple(
                items
                    .iter()
                    .map(|x| Key::of(x, span))
                    .collect::<Result<_>>()?,
            ),
            other => {
                let msg = format!("TypeError: unhashable type: '{}'", other.type_name());
                return Err(rt_err(span, msg));
            }
        })
    }
}

/// The entries in insertion order, and where each key's entry is.
#[derive(Debug, Default)]
pub(super) struct Table<'m> {
    entries: Vec<(Value<'m>, Value<'m>)>,
    index: HashMap<Key, usize>,
}

impl<'m> Table<'m> {
    pub fn new_ref(table: Table<'m>) -> TableRef<'m> {
        Rc::new(RefCell::new(table))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entry at insertion position `i`.
    pub fn entry(&self, i: usize) -> Option<&(Value<'m>, Value<'m>)> {
        self.entries.get(i)
    }

    pub fn entries(&self) -> &[(Value<'m>, Value<'m>)] {
        &self.entries
    }

    /// The value stored under `key`.
    pub fn get(&self, key: &Key) -> Option<&Value<'m>> {
        let i = *self.index.get(key)?;
        self.entries.get(i).map(|(_, v)| v)
    }

    pub fn contains(&self, key: &Key) -> bool {
        self.index.contains_key(key)
    }

    /// Stores `value` under `key`, whose value is `k`. A key already
    /// present keeps its place and its first key value, as in Python.
    pub fn insert(&mut self, key: Key, k: Value<'m>, value: Value<'m>) {
        if let Some(&i) = self.index.get(&key) {
            if let Some(entry) = self.entries.get_mut(i) {
                entry.1 = value;
            }
            return;
        }
        self.index.insert(key, self.entries.len());
        self.entries.push((k, value));
    }
}
