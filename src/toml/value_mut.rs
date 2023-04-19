use crate::toml::data::{Data, Id};
use crate::toml::raw::{self, Raw};

/// A mutable value inside of a document.
pub struct ValueMut<'a> {
    data: &'a mut Data,
    pub(crate) id: Id,
}

impl<'a> ValueMut<'a> {
    /// Construct a new mutable value.
    pub(crate) fn new(data: &'a mut Data, id: Id) -> Self {
        Self { data, id }
    }
}
