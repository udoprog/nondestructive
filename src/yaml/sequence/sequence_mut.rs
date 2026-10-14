use core::mem;

use alloc::vec::Vec;

use crate::yaml::data::{Data, Id, StringId};
use crate::yaml::raw::{self, Raw};
use crate::yaml::{Block, Separator, Sequence, ValueMut};

impl Data {
    /// Make `next` take the place of the removed first item `removed` of a
    /// collection.
    ///
    /// The first item has no separator in its prefix, while `next` has one.
    /// Everything up to and including the first line break in the prefix of
    /// `next` (and the indentation after it) belongs to the line of the removed
    /// item and is dropped, the rest (such as comments on lines of their own)
    /// is kept after the prefix of `removed`.
    pub(crate) fn take_first_prefix(&mut self, removed: Id, next: Id) {
        let next_prefix = self.prefix(next);

        let rest = match next_prefix.iter().position(|&b| b == b'\n') {
            Some(n) => {
                let rest = &next_prefix[n + 1..];
                let ws = rest
                    .iter()
                    .take_while(|&&b| matches!(b, b' ' | b'\t'))
                    .count();
                &rest[ws..]
            }
            None => &[][..],
        };

        let mut prefix = Vec::new();
        prefix.extend_from_slice(self.prefix(removed));
        prefix.extend_from_slice(rest);
        let prefix = self.insert_str(prefix);

        let raw = mem::replace(self.raw_mut(next), Raw::Null(raw::Null::Empty));
        self.replace_with(next, prefix, raw);
    }

    /// Prefix for an item appended to a flow collection whose first and last
    /// items are `first` and `last`, with the given inner suffix (the whitespace before the closing bracket).
    ///
    /// Copies the separator after the last `,`, keeping only the final line
    /// break and indentation of a multi-line one so that comments are not
    /// duplicated.
    pub(crate) fn flow_item_prefix(
        &mut self,
        first: Option<Id>,
        last: Option<Id>,
        suffix: StringId,
    ) -> StringId {
        let last = match (first, last) {
            (Some(first), Some(last)) if first == last && self.prefix(last).is_empty() => {
                return self.insert_str(" ");
            }
            (_, Some(last)) => self.layout(last).prefix,
            _ => {
                let blank = self.str(suffix).iter().all(|&b| matches!(b, b' ' | b'\t'));

                return if blank { suffix } else { self.insert_str("") };
            }
        };

        self.line_prefix(last).unwrap_or(last)
    }

    /// Split the prefix of `at` for an item inserted before it, returning the
    /// prefix of the new item, or `None` if the prefix has no line break.
    ///
    /// The inverse of [`Data::take_first_prefix`]: the text before the first
    /// line break (such as a comment trailing the previous item) stays on the
    /// line of the previous item, followed by the new item on a line of its own
    /// with the indentation of `at`. The rest, from the first line break on,
    /// stays with `at`.
    pub(crate) fn split_item_prefix(&mut self, at: Id) -> Option<StringId> {
        let prefix = self.layout(at).prefix;
        let line = self.line_prefix(prefix)?;

        let string = self.str(prefix);
        let n = string.iter().position(|&b| b == b'\n')?;
        let start = if n > 0 && string[n - 1] == b'\r' {
            n - 1
        } else {
            n
        };

        let mut head = string[..start].to_vec();
        let tail = string[start..].to_vec();
        head.extend_from_slice(self.str(line));

        let head = self.insert_str(head);
        let tail = self.insert_str(tail);
        self.set_prefix(at, tail);
        Some(head)
    }

    /// The final line break and indentation of the prefix `prefix`, or `None`
    /// if it has no line break.
    ///
    /// This drops comments and blank lines so that they are not duplicated
    /// when the prefix is reused for a new item.
    pub(crate) fn line_prefix(&mut self, prefix: StringId) -> Option<StringId> {
        let prefix = self.str(prefix);
        let n = prefix.iter().rposition(|&b| b == b'\n')?;

        let start = if n > 0 && prefix[n - 1] == b'\r' {
            n - 1
        } else {
            n
        };

        let indent = prefix[n + 1..]
            .iter()
            .take_while(|&&b| matches!(b, b' ' | b'\t'))
            .count();

        let prefix = prefix[start..n + 1 + indent].to_vec();
        Some(self.insert_str(prefix))
    }
}

/// Mutator for a sequence.
pub struct SequenceMut<'a> {
    data: &'a mut Data,
    pub(crate) id: Id,
}

macro_rules! push_float {
    ($name:ident, $insert:ident, $ty:ty, $string:literal, $lit:literal, $hint:ident) => {
        #[doc = concat!("Push the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use anyhow::Context;
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::from_slice(
        ///     r"
        ///     - 10
        ///     "
        /// )?;
        ///
        /// let mut value = doc.as_mut().into_sequence_mut().context("not a sequence")?;
        ///
        #[doc = concat!("value.", stringify!($name), "(", stringify!($lit), ");")]
        /// assert_eq!(
        ///     doc.to_string(),
        ///     r"
        ///     - 10
        #[doc = concat!("    - ", $lit)]
        ///     "
        /// );
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = ryu::Buffer::new();
            let number = self
                .data
                .insert_str(crate::yaml::value_mut::format_float(&mut buffer, value));
            let value = Raw::Number(raw::Number::new(number, crate::yaml::serde_hint::$hint));
            self.inner_push(Separator::Auto, value);
        }

        #[doc = concat!("Insert the value as a ", $string, " at `index`, shifting later items.")]
        ///
        /// # Panics
        ///
        /// Panics if `index` is greater than the length of the sequence.
        ///
        /// # Examples
        ///
        /// ```
        /// use anyhow::Context;
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::from_slice(
        ///     r"
        ///     - 10
        ///     - 20
        ///     "
        /// )?;
        ///
        /// let mut value = doc.as_mut().into_sequence_mut().context("not a sequence")?;
        ///
        #[doc = concat!("value.", stringify!($insert), "(1, ", stringify!($lit), ");")]
        ///
        /// assert_eq!(
        ///     doc.to_string(),
        ///     r"
        ///     - 10
        #[doc = concat!("    - ", $lit)]
        ///     - 20
        ///     "
        /// );
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        pub fn $insert(&mut self, index: usize, value: $ty) {
            let mut buffer = ryu::Buffer::new();
            let number = self
                .data
                .insert_str(crate::yaml::value_mut::format_float(&mut buffer, value));
            let value = Raw::Number(raw::Number::new(number, crate::yaml::serde_hint::$hint));
            self.inner_insert(index, Separator::Auto, value);
        }
    };
}

macro_rules! push_number {
    ($name:ident, $insert:ident, $ty:ty, $string:literal, $lit:literal, $hint:ident) => {
        #[doc = concat!("Push the value as a ", $string, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use anyhow::Context;
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::from_slice(
        ///     r"
        ///     - 10
        ///     "
        /// )?;
        ///
        /// let mut value = doc.as_mut().into_sequence_mut().context("not a sequence")?;
        ///
        #[doc = concat!("value.", stringify!($name), "(", stringify!($lit), ");")]
        ///
        /// assert_eq!(
        ///     doc.to_string(),
        ///     r"
        ///     - 10
        #[doc = concat!("    - ", stringify!($lit))]
        ///     "
        /// );
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        pub fn $name(&mut self, value: $ty) {
            let mut buffer = itoa::Buffer::new();
            let number = self.data.insert_str(buffer.format(value));
            let value = Raw::Number(raw::Number::new(number, crate::yaml::serde_hint::$hint));
            self.inner_push(Separator::Auto, value);
        }

        #[doc = concat!("Insert the value as a ", $string, " at `index`, shifting later items.")]
        ///
        /// # Panics
        ///
        /// Panics if `index` is greater than the length of the sequence.
        ///
        /// # Examples
        ///
        /// ```
        /// use anyhow::Context;
        /// use nondestructive::yaml;
        ///
        /// let mut doc = yaml::from_slice(
        ///     r"
        ///     - 10
        ///     - 20
        ///     "
        /// )?;
        ///
        /// let mut value = doc.as_mut().into_sequence_mut().context("not a sequence")?;
        ///
        #[doc = concat!("value.", stringify!($insert), "(1, ", stringify!($lit), ");")]
        ///
        /// assert_eq!(
        ///     doc.to_string(),
        ///     r"
        ///     - 10
        #[doc = concat!("    - ", stringify!($lit))]
        ///     - 20
        ///     "
        /// );
        /// # Ok::<_, anyhow::Error>(())
        /// ```
        pub fn $insert(&mut self, index: usize, value: $ty) {
            let mut buffer = itoa::Buffer::new();
            let number = self.data.insert_str(buffer.format(value));
            let value = Raw::Number(raw::Number::new(number, crate::yaml::serde_hint::$hint));
            self.inner_insert(index, Separator::Auto, value);
        }
    };
}

impl<'a> SequenceMut<'a> {
    pub(crate) fn new(data: &'a mut Data, id: Id) -> Self {
        Self { data, id }
    }

    /// Make insertion prefix.
    fn make_prefix(&mut self) -> StringId {
        let mut out = Vec::new();
        out.push(raw::NEWLINE);
        out.resize(
            self.data.sequence(self.id).indent.saturating_add(1),
            raw::SPACE,
        );
        self.data.insert_str(out)
    }

    /// Push a value on the sequence.
    fn inner_push(&mut self, separator: Separator, value: Raw) -> Id {
        let sequence = self.data.sequence(self.id);

        let (item_prefix, flow) = if let raw::SequenceKind::Inline { suffix, .. } = sequence.kind {
            let (first, last) = (
                sequence.items.first().copied(),
                sequence.items.last().copied(),
            );
            (self.data.flow_item_prefix(first, last, suffix), true)
        } else if sequence.items.last().is_some() {
            (self.make_prefix(), false)
        } else {
            (self.data.insert_str(""), false)
        };

        let item_id = self
            .data
            .insert(Raw::Null(raw::Null::Empty), item_prefix, Some(self.id));

        let value_prefix = match separator {
            Separator::Auto => match self.data.sequence(self.id).items.last() {
                Some(last) => {
                    self.data
                        .layout(self.data.sequence_item(*last).value)
                        .prefix
                }
                None if flow => self.data.insert_str(""),
                None => self.data.insert_str(" "),
            },
            Separator::Custom(separator) => self.data.insert_str(separator),
        };

        let value = self.data.insert(value, value_prefix, Some(item_id));

        let separator_prefix = self.data.insert_str("");
        self.data.replace(
            item_id,
            Raw::SequenceItem(raw::SequenceItem {
                value,
                separator_prefix,
            }),
        );
        self.data.sequence_mut(self.id).items.push(item_id);
        value
    }

    /// Insert a value at `index`, keeping the formatting of the surrounding
    /// items.
    ///
    /// The new item copies the line break and indentation (block) or the
    /// separators (flow) of its neighbours, while comments stay with the item
    /// they precede.
    fn inner_insert(&mut self, index: usize, separator: Separator, value: Raw) -> Id {
        let sequence = self.data.sequence(self.id);
        let len = sequence.items.len();

        assert!(
            index <= len,
            "insertion index (is {index}) should be <= len (is {len})"
        );

        if index == len {
            return self.inner_push(separator, value);
        }

        let at = sequence.items[index];
        let before = index.checked_sub(1).map(|i| sequence.items[i]);
        let after = sequence.items.get(1).copied();
        let kind = sequence.kind.clone();
        let at_prefix = self.data.layout(at).prefix;

        let (item_prefix, separator_prefix) = match kind {
            raw::SequenceKind::Mapping => {
                let item_prefix = if before.is_some() {
                    match self.data.split_item_prefix(at) {
                        Some(prefix) => prefix,
                        None => self.make_prefix(),
                    }
                } else {
                    let line =
                        after.and_then(|n| self.data.line_prefix(self.data.layout(n).prefix));

                    let line = match line {
                        Some(line) => line,
                        None => self.make_prefix(),
                    };

                    self.data.set_prefix(at, line);
                    at_prefix
                };

                (item_prefix, self.data.insert_str(""))
            }
            raw::SequenceKind::Inline { trailing, suffix } => {
                // An item followed by a `,` whose separator the new item copies.
                let followed = match before {
                    Some(before) => Some(before),
                    None if after.is_some() || trailing => Some(at),
                    None => None,
                };

                let item_prefix = if before.is_some() {
                    self.data.split_item_prefix(at).unwrap_or(at_prefix)
                } else {
                    let prefix = self
                        .data
                        .flow_item_prefix(Some(at), after.or(Some(at)), suffix);
                    self.data.set_prefix(at, prefix);
                    at_prefix
                };

                let separator_prefix = followed
                    .map(|id| self.data.sequence_item(id).separator_prefix)
                    .filter(|&id| self.data.str(id).iter().all(|&b| matches!(b, b' ' | b'\t')));

                let separator_prefix = match separator_prefix {
                    Some(separator_prefix) => separator_prefix,
                    None => self.data.insert_str(""),
                };

                (item_prefix, separator_prefix)
            }
        };

        let value_prefix = match separator {
            Separator::Auto => self.data.layout(self.data.sequence_item(at).value).prefix,
            Separator::Custom(separator) => self.data.insert_str(separator),
        };

        let item_id = self
            .data
            .insert(Raw::Null(raw::Null::Empty), item_prefix, Some(self.id));
        let value = self.data.insert(value, value_prefix, Some(item_id));

        self.data.replace(
            item_id,
            Raw::SequenceItem(raw::SequenceItem {
                value,
                separator_prefix,
            }),
        );

        self.data.sequence_mut(self.id).items.insert(index, item_id);
        value
    }

    /// Coerce a mutable sequence as an immutable [Sequence].
    ///
    /// This is useful to be able to directly use methods only available on
    /// [Sequence].
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     - one
    ///     - two
    ///     - three
    ///     ",
    /// )?;
    ///
    /// let root = doc.as_mut().into_sequence_mut().context("missing root sequence")?;
    /// let root = root.as_ref();
    ///
    /// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    /// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
    /// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn as_ref(&self) -> Sequence<'_> {
        Sequence::new(self.data, self.id)
    }

    /// Coerce a mutable sequence into an immutable [Sequence] with the lifetime of the
    /// current reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     - one
    ///     - two
    ///     - three
    ///     ",
    /// )?;
    ///
    /// let root = doc.as_mut().into_sequence_mut().context("missing root sequence")?.into_ref();
    ///
    /// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    /// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
    /// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn into_ref(self) -> Sequence<'a> {
        Sequence::new(self.data, self.id)
    }

    /// Get a value mutably from the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     - 10
    ///     - 20
    ///     - inner: 400
    ///     - "I am a quoted string!"
    ///     "#
    /// )?;
    ///
    /// let mut root = doc.as_mut();
    /// let mut root = root.as_sequence_mut().context("missing root sequence")?;
    /// root.get_mut(1).context("missing inner sequence")?.set_u32(30);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - 10
    ///     - 30
    ///     - inner: 400
    ///     - "I am a quoted string!"
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn get_mut(&mut self, index: usize) -> Option<ValueMut<'_>> {
        if let Some(item) = self.data.sequence(self.id).items.get(index) {
            let item = self.data.sequence_item(*item);
            return Some(ValueMut::new(self.data, item.value));
        }

        None
    }

    /// Get a value mutably from the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     - 10
    ///     - 20
    ///     - inner: 400
    ///     - "I am a quoted string!"
    ///     "#
    /// )?;
    ///
    /// let mut root = doc.as_mut();
    /// let mut value = root.as_sequence_mut().and_then(|v| v.get_into_mut(1)).context("missing value")?;
    /// value.set_u32(30);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - 10
    ///     - 30
    ///     - inner: 400
    ///     - "I am a quoted string!"
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn get_into_mut(self, index: usize) -> Option<ValueMut<'a>> {
        if let Some(item) = self.data.sequence(self.id).items.get(index) {
            let item = self.data.sequence_item(*item);
            return Some(ValueMut::new(self.data, item.value));
        }

        None
    }

    /// Remove the given index from the sequence, returning a boolean indicating if
    /// it existed in the sequence or not.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     - 10
    ///     - 20
    ///     - inner: 400
    ///     - "I am a quoted string!"
    ///     "#
    /// )?;
    ///
    /// let mut root = doc.as_mut();
    /// let mut root = root.as_sequence_mut().context("missing root sequence")?;
    ///
    /// assert!(!root.remove(4));
    /// assert!(root.remove(2));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r#"
    ///     - 10
    ///     - 20
    ///     - "I am a quoted string!"
    ///     "#
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn remove(&mut self, index: usize) -> bool {
        let raw = self.data.sequence_mut(self.id);

        if raw.items.len() <= index {
            return false;
        }

        let item = raw.items.remove(index);

        if index == 0 {
            if let Some(&next) = raw.items.first() {
                self.data.take_first_prefix(item, next);
            }
        }

        self.data.drop(item);
        true
    }

    /// Clear all the elements in a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r#"
    ///     - 10
    ///     - 20
    ///     - inner: 400
    ///     - "I am a quoted string!"
    ///     "#
    /// )?;
    ///
    /// let mut root = doc.as_mut();
    /// let mut root = root.as_sequence_mut().context("missing root sequence")?;
    ///
    /// root.clear();
    ///
    /// assert_eq!(doc.to_string(), "\n    \n    ");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn clear(&mut self) {
        let mut items = mem::take(&mut self.data.sequence_mut(self.id).items);

        for item in items.drain(..) {
            self.data.drop(item);
        }

        self.data.sequence_mut(self.id).items = items;
    }

    /// Push a new null value and return a [`ValueMut`] to the newly pushed value.
    ///
    /// This allows for setting a custom [`Separator`].
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     - one
    ///     - two
    ///     ",
    /// )?;
    ///
    /// let mut root = doc.as_mut().into_sequence_mut().context("missing root sequence")?;
    /// root.push(yaml::Separator::Custom("   ")).set_bool(true);
    ///
    /// assert_eq! {
    ///     doc.to_string(),
    ///     r"
    ///     - one
    ///     - two
    ///     -   true
    ///     "
    /// };
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn push(&mut self, separator: Separator<'_>) -> ValueMut<'_> {
        let value = self.inner_push(separator, Raw::Null(raw::Null::Empty));
        ValueMut::new(self.data, value)
    }

    /// Insert a new null value at `index`, shifting later items, and return a
    /// [`ValueMut`] to the inserted value.
    ///
    /// The new item follows the formatting of its neighbours in both block and
    /// flow sequences, and comments stay with the item they precede. This
    /// allows for setting a custom [`Separator`].
    ///
    /// # Panics
    ///
    /// Panics if `index` is greater than the length of the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     - one
    ///     ## about two
    ///     - two
    ///     ",
    /// )?;
    ///
    /// let mut root = doc.as_mut().into_sequence_mut().context("missing root sequence")?;
    /// root.insert(0, yaml::Separator::Auto).set_string("zero");
    /// root.insert(2, yaml::Separator::Custom("   ")).set_bool(true);
    ///
    /// assert_eq! {
    ///     doc.to_string(),
    ///     r"
    ///     - zero
    ///     - one
    ///     -   true
    ///     ## about two
    ///     - two
    ///     "
    /// };
    ///
    /// let mut doc = yaml::from_slice("[one, two]")?;
    /// let mut root = doc.as_mut().into_sequence_mut().context("missing root sequence")?;
    /// root.insert(1, yaml::Separator::Auto).set_u32(42);
    /// assert_eq!(doc.to_string(), "[one, 42, two]");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn insert(&mut self, index: usize, separator: Separator<'_>) -> ValueMut<'_> {
        let value = self.inner_insert(index, separator, Raw::Null(raw::Null::Empty));
        ValueMut::new(self.data, value)
    }

    /// Push a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     - - 10
    ///     "
    /// )?;
    ///
    /// let mut value = doc.as_mut().into_sequence_mut().context("not a sequence")?;
    /// let mut value = value.get_mut(0).and_then(|v| v.into_sequence_mut()).expect("missing inner");
    /// value.push_string("nice string");
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     - - 10
    ///       - nice string
    ///     "
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn push_string<S>(&mut self, string: S)
    where
        S: AsRef<str>,
    {
        let string = raw::new_string(self.data, string);
        self.inner_push(Separator::Auto, string);
    }

    /// Insert a string at `index`, shifting later items.
    ///
    /// # Panics
    ///
    /// Panics if `index` is greater than the length of the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     - - 10
    ///     "
    /// )?;
    ///
    /// let mut value = doc.as_mut().into_sequence_mut().context("not a sequence")?;
    /// let mut value = value.get_mut(0).and_then(|v| v.into_sequence_mut()).expect("missing inner");
    /// value.insert_string(0, "nice string");
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     - - nice string
    ///       - 10
    ///     "
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn insert_string<S>(&mut self, index: usize, string: S)
    where
        S: AsRef<str>,
    {
        let string = raw::new_string(self.data, string);
        self.inner_insert(index, Separator::Auto, string);
    }

    /// Push a value as a literal or folded block.
    ///
    /// This takes an iterator, which will be used to construct the block. The
    /// underlying value type produced is in fact a string, and can be read
    /// through methods such as [`Value::as_str`][crate::yaml::Value::as_str].
    ///
    /// Each item is one line of the block, and an item containing line breaks
    /// becomes several lines. The value is what a YAML parser reads from the
    /// written block: folded lines are joined by spaces, and trailing empty
    /// lines are dropped unless the block uses
    /// [`Chomp::Keep`][crate::yaml::Chomp::Keep]. If the first non-empty line
    /// starts with whitespace, an indentation indicator such as `|2` is written
    /// so that the whitespace stays part of the value.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     string
    ///     "
    /// )?;
    ///
    /// let mut sequence = doc.as_mut().make_sequence();
    /// sequence.clear();
    /// sequence.push_block(["foo", "bar", "baz"], yaml::Block::Literal(yaml::Chomp::Clip));
    /// assert_eq!(sequence.as_ref().last().and_then(|v| v.as_str()), Some("foo\nbar\nbaz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     - |
    ///       foo
    ///       bar
    ///       baz
    ///     "
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_sequence();
    /// sequence.clear();
    /// sequence.push_block(["foo", "bar", "baz"], yaml::Block::Literal(yaml::Chomp::Keep));
    /// assert_eq!(sequence.as_ref().last().and_then(|v| v.as_str()), Some("foo\nbar\nbaz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     - |+
    ///       foo
    ///       bar
    ///       baz
    ///     "
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_sequence();
    /// sequence.clear();
    /// sequence.push_block(["foo", "bar", "baz"], yaml::Block::Literal(yaml::Chomp::Strip));
    /// assert_eq!(sequence.as_ref().last().and_then(|v| v.as_str()), Some("foo\nbar\nbaz"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     - |-
    ///       foo
    ///       bar
    ///       baz
    ///     "
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_sequence();
    /// sequence.clear();
    /// sequence.push_block(["foo", "bar", "baz"], yaml::Block::Folded(yaml::Chomp::Clip));
    /// assert_eq!(sequence.as_ref().last().and_then(|v| v.as_str()), Some("foo bar baz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     - >
    ///       foo
    ///       bar
    ///       baz
    ///     "
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_sequence();
    /// sequence.clear();
    /// sequence.push_block(["foo", "bar", "baz"], yaml::Block::Folded(yaml::Chomp::Keep));
    /// assert_eq!(sequence.as_ref().last().and_then(|v| v.as_str()), Some("foo bar baz\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     - >+
    ///       foo
    ///       bar
    ///       baz
    ///     "
    /// );
    ///
    /// let mut sequence = doc.as_mut().make_sequence();
    /// sequence.clear();
    /// sequence.push_block(["foo", "bar", "baz"], yaml::Block::Folded(yaml::Chomp::Strip));
    /// assert_eq!(sequence.as_ref().last().and_then(|v| v.as_str()), Some("foo bar baz"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     - >-
    ///       foo
    ///       bar
    ///       baz
    ///     "
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn push_block<I>(&mut self, iter: I, block: Block)
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let value = raw::new_block(self.data, self.id, iter, block);
        let id = self.inner_push(Separator::Auto, value);
        raw::settle_block(self.data, id);
    }

    /// Insert a value as a literal or folded block at `index`, shifting later
    /// items.
    ///
    /// The block is built as in [`SequenceMut::push_block`].
    ///
    /// # Panics
    ///
    /// Panics if `index` is greater than the length of the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     - one
    ///     - two
    ///     "
    /// )?;
    ///
    /// let mut sequence = doc.as_mut().into_sequence_mut().context("not a sequence")?;
    /// sequence.insert_block(1, ["foo", "bar"], yaml::Block::Literal(yaml::Chomp::Clip));
    /// assert_eq!(sequence.as_ref().get(1).and_then(|v| v.as_str()), Some("foo\nbar\n"));
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     - one
    ///     - |
    ///       foo
    ///       bar
    ///     - two
    ///     "
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn insert_block<I>(&mut self, index: usize, iter: I, block: Block)
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let value = raw::new_block(self.data, self.id, iter, block);
        let id = self.inner_insert(index, Separator::Auto, value);
        raw::settle_block(self.data, id);
    }

    /// Push a bool.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice(
    ///     r"
    ///     - - 10
    ///     "
    /// )?;
    ///
    /// let mut value = doc.as_mut().into_sequence_mut().context("not a sequence")?;
    /// let mut value = value.get_mut(0).and_then(|v| v.into_sequence_mut()).expect("missing inner");
    /// value.push_bool(false);
    ///
    /// assert_eq!(
    ///     doc.to_string(),
    ///     r"
    ///     - - 10
    ///       - false
    ///     "
    /// );
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn push_bool(&mut self, value: bool) {
        let value = raw::new_bool(self.data, value);
        self.inner_push(Separator::Auto, value);
    }

    /// Insert a bool at `index`, shifting later items.
    ///
    /// # Panics
    ///
    /// Panics if `index` is greater than the length of the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Context;
    /// use nondestructive::yaml;
    ///
    /// let mut doc = yaml::from_slice("[1, 2]")?;
    ///
    /// let mut value = doc.as_mut().into_sequence_mut().context("not a sequence")?;
    /// value.insert_bool(0, false);
    /// value.insert_bool(3, true);
    ///
    /// assert_eq!(doc.to_string(), "[false, 1, 2, true]");
    /// # Ok::<_, anyhow::Error>(())
    /// ```
    pub fn insert_bool(&mut self, index: usize, value: bool) {
        let value = raw::new_bool(self.data, value);
        self.inner_insert(index, Separator::Auto, value);
    }

    push_float!(push_f32, insert_f32, f32, "32-bit float", 10.42, F32);
    push_float!(push_f64, insert_f64, f64, "64-bit float", 10.42, F64);
    push_number!(push_u8, insert_u8, u8, "8-bit unsigned integer", 42, U8);
    push_number!(push_i8, insert_i8, i8, "8-bit signed integer", -42, I8);
    push_number!(
        push_u16,
        insert_u16,
        u16,
        "16-bit unsigned integer",
        42,
        U16
    );
    push_number!(push_i16, insert_i16, i16, "16-bit signed integer", -42, I16);
    push_number!(
        push_u32,
        insert_u32,
        u32,
        "32-bit unsigned integer",
        42,
        U32
    );
    push_number!(push_i32, insert_i32, i32, "32-bit signed integer", -42, I32);
    push_number!(
        push_u64,
        insert_u64,
        u64,
        "64-bit unsigned integer",
        42,
        U64
    );
    push_number!(push_i64, insert_i64, i64, "64-bit signed integer", -42, I64);
    push_number!(
        push_u128,
        insert_u128,
        u128,
        "128-bit unsigned integer",
        42,
        U128
    );
    push_number!(
        push_i128,
        insert_i128,
        i128,
        "128-bit signed integer",
        -42,
        I128
    );
}

/// Formats the same as the immutable view of the sequence.
///
/// # Examples
///
/// ```
/// use anyhow::Context;
/// use nondestructive::yaml;
///
/// let mut doc = yaml::from_slice("[1, 2]")?;
/// let mut root = doc.as_mut();
/// let root = root.as_sequence_mut().context("missing root sequence")?;
/// assert_eq!(format!("{root:?}"), "[Value(1), Value(2)]");
/// # Ok::<_, anyhow::Error>(())
/// ```
impl core::fmt::Debug for SequenceMut<'_> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.as_ref(), f)
    }
}
