use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("invalid item id format: {0}")]
    InvalidId(String),

    #[error("invalid content kind integer code: {0}")]
    InvalidContentKind(i32),

    #[error("unknown content kind label: '{0}'")]
    UnknownKindLabel(String),
}
