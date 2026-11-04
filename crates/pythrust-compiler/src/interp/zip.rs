//! `enumerate(...)` and `zip(...)` (D51, D58): never values. Each is
//! consumed where it is written, a step at a time and in CPython's
//! order, so a `for` body that appends to a list extends the loop.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use super::ops::overflow;
use super::value::{Iter, Outcome, Value};
use super::{rt_err, tasks, Exec};
use crate::span::Span;

/// The state of one `enumerate(...)` or `zip(...)`.
pub(super) struct Zipped<'m> {
    sources: Vec<Source<'m>>,
    /// `enumerate`'s next index, wide so the one past `i64::MAX` is
    /// refused only when an item would get it; `None` for `zip`.
    index: Option<i128>,
    span: Span,
}

/// One iterable a [`Zipped`] reads.
pub(super) enum Source<'m> {
    /// What `for` iterates, the argument at the span.
    Items(Iter<'m>, Span),
    /// `map`'s results: each step takes one.
    Results(Rc<RefCell<VecDeque<Outcome<'m>>>>),
    /// A nested `enumerate(...)` or `zip(...)` (D58).
    Zip(Box<Zipped<'m>>),
}

impl<'m> Zipped<'m> {
    /// `enumerate(source, start)`.
    pub fn enumerate(source: Source<'m>, start: i64, span: Span) -> Self {
        Zipped {
            sources: vec![source],
            index: Some(i128::from(start)),
            span,
        }
    }

    /// `zip(sources...)`.
    pub fn zip(sources: Vec<Source<'m>>, span: Span) -> Self {
        Zipped {
            sources,
            index: None,
            span,
        }
    }

    /// The next tuple, or `None` once a source is exhausted. A `zip`
    /// pulls its sources left to right and stops at the first that has
    /// nothing left; one of none gives nothing.
    pub fn step(&mut self) -> Exec<'m, Option<Value<'m>>> {
        if self.sources.is_empty() {
            return Ok(None);
        }
        let mut items = Vec::with_capacity(self.sources.len() + 1);
        for source in &mut self.sources {
            match source.next()? {
                Some(v) => items.push(v),
                None => return Ok(None),
            }
        }
        // As CPython's, the index comes after the item it numbers.
        if let Some(i) = &mut self.index {
            let n = i64::try_from(*i).map_err(|_| overflow(self.span))?;
            items.insert(0, Value::Int(n));
            *i += 1;
        }
        Ok(Some(Value::Tuple(Rc::from(items))))
    }

    /// Every tuple left, as a list.
    pub fn drain(&mut self) -> Exec<'m, Value<'m>> {
        let mut items = Vec::new();
        while let Some(v) = self.step()? {
            items.push(v);
        }
        Ok(Value::list(items))
    }
}

impl<'m> Source<'m> {
    fn next(&mut self) -> Exec<'m, Option<Value<'m>>> {
        match self {
            Source::Items(items, span) => match items.next() {
                Some(v) => Ok(Some(v)),
                None => match items.changed_size() {
                    Some(msg) => Err(rt_err(*span, format!("RuntimeError: {msg}")).into()),
                    None => Ok(None),
                },
            },
            Source::Results(results) => tasks::take(results),
            Source::Zip(z) => z.step(),
        }
    }
}
