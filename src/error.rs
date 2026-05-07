use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Config(String),
    Git(String),
    Search(String),
    Llm(String),
    Db(String),
    Note(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "IO error: {e}"),
            AppError::Config(s) => write!(f, "Config error: {s}"),
            AppError::Git(s) => write!(f, "Git error: {s}"),
            AppError::Search(s) => write!(f, "Search error: {s}"),
            AppError::Llm(s) => write!(f, "LLM error: {s}"),
            AppError::Db(s) => write!(f, "DB error: {s}"),
            AppError::Note(s) => write!(f, "Note error: {s}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}
