use std::error::Error;
use std::fmt;

/// id 単体の検証エラー。検証ルール自体は `Id::validate` が持つ。
#[derive(Debug, PartialEq)]
pub enum IdError {
    Empty,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdError::Empty => write!(f, "empty id is not allowed"),
        }
    }
}

impl Error for IdError {}

#[derive(Debug, PartialEq)]
pub enum ForestError {
    DuplicateId { id: String },
    InvalidId { id: String, error: IdError },
    DanglingParent { id: String, parent: String },
    Cycle { id: String },
}

impl fmt::Display for ForestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ForestError::DuplicateId { id } => write!(f, "duplicate id: '{id}'"),
            ForestError::InvalidId { id, error } => {
                write!(f, "node '{id}' has invalid id: {error}")
            }
            ForestError::DanglingParent { id, parent } => {
                write!(f, "node '{id}' has dangling parent '{parent}'")
            }
            ForestError::Cycle { id } => {
                write!(f, "cycle detected at node '{id}'")
            }
        }
    }
}

impl Error for ForestError {
    // InvalidId は下層の IdError を原因として連鎖させる
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ForestError::InvalidId { error, .. } => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ValidationError {
    Catalog(ForestError),
    Taxonomy(ForestError),
    CategoryKeyNotRoot { resource: String, key: String },
    CategoryValueMissing { resource: String, value: String },
    CategoryValueWrongAxis { resource: String, key: String, value: String },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::Catalog(e) => write!(f, "catalog: {e}"),
            ValidationError::Taxonomy(e) => write!(f, "taxonomy: {e}"),
            ValidationError::CategoryKeyNotRoot { resource, key } => {
                write!(f, "resource '{resource}': category key '{key}' is not a root")
            }
            ValidationError::CategoryValueMissing { resource, value } => {
                write!(f, "resource '{resource}': category value '{value}' does not exist")
            }
            ValidationError::CategoryValueWrongAxis { resource, key, value } => {
                write!(
                    f,
                    "resource '{resource}': category value '{value}' does not belong to axis '{key}'"
                )
            }
        }
    }
}

impl Error for ValidationError {
    // 森由来のエラーは下層の ForestError を原因として連鎖させる
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ValidationError::Catalog(e) | ValidationError::Taxonomy(e) => Some(e),
            _ => None,
        }
    }
}

/// インポート境界でのパースエラー。JSON 不正・バージョン不一致・構造不整合の3種を区別する。
#[derive(Debug)]
pub enum ParseError {
    /// serde 由来のデシリアライズ失敗。JSON に限らない（今後 JSON 以外もあり得る）。
    Serde(String),
    UnsupportedVersion(u32),
    /// 構造検証（forest + categories クロス整合）で収集されたエラー群。
    Invalid(Errors<ValidationError>),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Serde(msg) => write!(f, "deserialize error: {msg}"),
            ParseError::UnsupportedVersion(v) => write!(f, "unsupported version: {v}"),
            ParseError::Invalid(errs) => write!(f, "invalid data: {errs}"),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseError::Invalid(errs) => Some(errs),
            _ => None,
        }
    }
}

/// 全件収集した検証エラーの集約。
/// `Vec` は `std::error::Error` を実装しないため、エラーとして扱える型でラップする。
#[derive(Debug, PartialEq)]
pub struct Errors<E>(pub Vec<E>);

impl<E> std::ops::Deref for Errors<E> {
    type Target = Vec<E>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E> IntoIterator for Errors<E> {
    type Item = E;
    type IntoIter = std::vec::IntoIter<E>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<E: fmt::Display> fmt::Display for Errors<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, e) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{e}")?;
        }
        Ok(())
    }
}

impl<E: Error> Error for Errors<E> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_is_usable_as_std_error() {
        let errs = Errors(vec![ForestError::DuplicateId { id: "x".into() }]);
        let _boxed: Box<dyn Error> = Box::new(errs);
    }

    #[test]
    fn invalid_id_chains_to_id_error_as_source() {
        let e = ForestError::InvalidId {
            id: "".into(),
            error: IdError::Empty,
        };
        let src = e.source().expect("source should be the underlying IdError");
        assert!(src.downcast_ref::<IdError>().is_some());
    }

    #[test]
    fn validation_error_chains_to_forest_error_as_source() {
        let e = ValidationError::Taxonomy(ForestError::Cycle { id: "a".into() });
        let src = e.source().expect("source should be the underlying ForestError");
        assert!(src.downcast_ref::<ForestError>().is_some());
    }

    #[test]
    fn display_lists_every_error() {
        let errs = Errors(vec![
            ForestError::DuplicateId { id: "a".into() },
            ForestError::Cycle { id: "b".into() },
        ]);
        let text = errs.to_string();
        assert!(text.contains("'a'"));
        assert!(text.contains("'b'"));
    }

}
