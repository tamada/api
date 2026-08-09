use std::path::PathBuf;

use thiserror::Error;

pub mod github;
pub mod product;
pub mod updater;

pub use product::{BaseProduct, InputItem, License, Link, LinkSpec, Product, Release};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("multiple errors: {}", format_errors(.0))]
    Array(Vec<Error>),
    #[error("{0}")]
    Clap(clap::Error),
    #[error("io error: {0}")]
    Io(std::io::Error),
    #[error("github error: {0}")]
    GitHub(String),
    #[error("parse error: {0}")]
    Parse(serde_json::Error),
    #[error("fatal error: {0}")]
    Fatal(String),
    #[error("{0}: already exists")]
    FileExists(PathBuf),
    #[error("{0}: file not found")]
    FileNotFound(PathBuf),
}

fn format_errors(errors: &[Error]) -> String {
    errors
        .iter()
        .map(|e| format!(" - {}", e))
        .collect::<Vec<_>>()
        .join("\n")
}

impl Error {
    pub fn from<T>(ok: T, arr: Vec<Error>) -> Result<T> {
        if arr.is_empty() {
            Ok(ok)
        } else if arr.len() == 1 {
            Err(arr.into_iter().next().unwrap())
        } else {
            Err(Error::Array(arr))
        }
    }
}
