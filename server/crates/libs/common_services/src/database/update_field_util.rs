#![allow(clippy::option_if_let_else)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(untagged)]
pub enum UpdateField<T> {
    Value(T),
    SetNull,
    #[serde(skip)]
    #[default]
    Ignore,
}

impl<T> UpdateField<T> {
    pub const fn is_ignore(&self) -> bool {
        matches!(self, Self::Ignore)
    }

    pub const fn not_ignore(&self) -> bool {
        !Self::is_ignore(self)
    }

    pub fn value(self) -> Option<T> {
        match self {
            Self::Value(v) => Some(v),
            _ => None,
        }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> UpdateField<U> {
        match self {
            Self::Value(v) => UpdateField::Value(f(v)),
            Self::SetNull => UpdateField::SetNull,
            Self::Ignore => UpdateField::Ignore,
        }
    }
}
