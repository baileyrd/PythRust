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
    /// An `int`-keyed table: a `bool` stored as a key is stored as its int.
    ints: bool,
}

impl<'m> Table<'m> {
    /// An empty table, `int`-keyed if `ints`.
    pub fn keyed(ints: bool) -> Self {
        Table {
            ints,
            ..Table::default()
        }
    }

    /// Makes the table `int`-keyed, turning any `bool` key into its int.
    pub fn int_keyed(&mut self) {
        if self.ints {
            return;
        }
        self.ints = true;
        for (k, _) in &mut self.entries {
            if let Value::Bool(b) = k {
                *k = Value::Int(i64::from(*b));
            }
        }
    }

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
    /// present keeps its place and its first key value, as in Python; a
    /// `bool` key of an `int`-keyed table is stored as its int.
    /// Only an `int`-keyed table can hold an `int`, so storing one makes
    /// the table `int`-keyed.
    pub fn insert(&mut self, key: Key, k: Value<'m>, value: Value<'m>) {
        if matches!(k, Value::Int(_)) && !self.ints {
            self.int_keyed();
        }
        if let Some(&i) = self.index.get(&key) {
            if let Some(entry) = self.entries.get_mut(i) {
                entry.1 = value;
            }
            return;
        }
        let k = match k {
            Value::Bool(b) if self.ints => Value::Int(i64::from(b)),
            k => k,
        };
        self.index.insert(key, self.entries.len());
        self.entries.push((k, value));
    }
}
