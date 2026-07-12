use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::error::IdError;

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id<T>(pub String, #[serde(skip)] PhantomData<fn() -> T>);

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Id(self.0.clone(), PhantomData)
    }
}

impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Id").field(&self.0).finish()
    }
}

impl<T> Id<T> {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn validate(&self) -> Result<(), IdError> {
        if self.0.is_empty() {
            return Err(IdError::Empty);
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn new_unchecked(s: impl Into<String>) -> Self {
        Id(s.into(), PhantomData)
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Id<T> {}

impl<T> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T> TryFrom<String> for Id<T> {
    type Error = IdError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        let id = Id(s, PhantomData);
        id.validate()?;
        Ok(id)
    }
}

impl<T> TryFrom<&str> for Id<T> {
    type Error = IdError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Id::try_from(s.to_string())
    }
}

impl<T> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> std::ops::Deref for Id<T> {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl<T> std::borrow::Borrow<str> for Id<T> {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_from_empty_str_is_err() {
        assert_eq!(Id::<()>::try_from(""), Err(IdError::Empty));
    }

    #[test]
    fn try_from_non_empty_str_is_ok() {
        assert!(Id::<()>::try_from("x").is_ok());
    }

    #[test]
    fn try_from_empty_string_is_err() {
        assert_eq!(Id::<()>::try_from(String::new()), Err(IdError::Empty));
    }

    #[test]
    fn try_from_non_empty_string_is_ok() {
        assert!(Id::<()>::try_from("hello".to_string()).is_ok());
    }
}
