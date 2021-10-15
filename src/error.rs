use thiserror::Error;

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum SnippextError {

    #[error("Config error: `{0}`")]
    ConfigError(#[from] config::ConfigError),

    /// Error variant that represents errors coming out of libgit2.
    #[error("Git error: `{0}`")]
    GitError(#[from] git2::Error),

    /// Settings validation errors
    #[error("Settings error: `{0:?}`")]
    ValidationError(Vec<String>)
}

// TODO: move this to lib?
pub type Result<T> = core::result::Result<T, SnippextError>;
