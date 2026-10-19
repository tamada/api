use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::product::Product;
use crate::{Error, Result};

/// The values of the previous build, keyed by the owner and the name of the
/// product.
///
/// The cache is read from the published API itself
/// (`https://tamada.github.io/api/products/index.json`, which serves the
/// `data/products.json` of the previous build as it is), or from a local JSON
/// file of the same shape. Only the fields that come from GitHub are reused
/// from it, and only for the products whose repository has not been modified
/// since; see [`crate::product::build_cached_product`].
///
/// The cache never decides anything else: the set of the products and their
/// configuration come from `assets/base-products.pkl` alone, so a product
/// dropped from the Pkl file disappears from the output even while the cache
/// still holds it.
#[derive(Debug, Default)]
pub struct Cache {
    entries: HashMap<(String, String), Product>,
}

impl Cache {
    /// A cache holding nothing, which makes every product be fetched.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Loads the cache from a URL (`http://` or `https://`) or a local file.
    pub fn load(source: &str) -> Result<Self> {
        let text = if is_url(source) {
            fetch(source)?
        } else {
            read_file(source)?
        };
        Self::parse(&text)
    }

    /// Loads the cache, falling back to an empty one when the source cannot be
    /// read or parsed. An unusable cache only costs a full fetch, hence it must
    /// never break a build: the first run has nothing published yet, and a
    /// deploy in flight may answer with anything.
    pub fn load_or_empty(source: &str) -> Self {
        match Self::load(source) {
            Ok(cache) => {
                log::info!("{}: {} cached products", source, cache.len());
                cache
            }
            Err(e) => {
                log::warn!(
                    "{}: unusable cache ({}), hence every product is fetched",
                    source,
                    e
                );
                Self::empty()
            }
        }
    }

    /// Parses the JSON array of the products of the previous build. An entry
    /// that cannot be read is skipped with a warning, so that one broken
    /// product does not cost the whole cache.
    pub fn parse(text: &str) -> Result<Self> {
        let values: Vec<Value> = serde_json::from_str(text).map_err(Error::Parse)?;
        let mut entries = HashMap::new();
        for value in values {
            match serde_json::from_value::<Product>(value) {
                Ok(product) => {
                    entries.insert((product.owner.clone(), product.name.clone()), product);
                }
                Err(e) => log::warn!("unreadable cache entry, skipped: {}", e),
            }
        }
        Ok(Self { entries })
    }

    pub fn get(&self, owner: &str, name: &str) -> Option<&Product> {
        self.entries.get(&(owner.to_string(), name.to_string()))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn is_url(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://")
}

fn fetch(url: &str) -> Result<String> {
    log::info!("fetching the cache from {}", url);
    let output = Command::new("curl")
        .args(["-fsSL", url])
        .output()
        .map_err(Error::Io)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(Error::Fatal(format!(
            "curl {}: {}",
            url,
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn read_file(path: &str) -> Result<String> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(Error::FileNotFound(path.to_path_buf()));
    }
    std::fs::read_to_string(path).map_err(Error::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CACHE_JSON: &str = r#"[
        {"owner":"tamada","name":"totebag","description":"archiver",
         "releases":[{"name":"v0.10.0","release_date":"2026-10-01T00:00:00Z","description":""}],
         "license":[{"spdx":"mit","name":"MIT License","url":null}],
         "links":[{"label":"www","url":"https://totebag.example.com"},
                  {"label":"repository","url":"https://github.com/tamada/totebag"}],
         "languages":["Rust"],"topics":["archive"],
         "last_updated":"2026-10-12T00:13:25Z"},
        {"owner":"tamada","name":"fauxrest","description":"a static REST API builder",
         "releases":[],"license":[],"links":[],"languages":["Rust"],"topics":[],
         "last_updated":"2026-10-10T00:26:00Z"}
    ]"#;

    #[test]
    fn parse_the_published_products() -> Result<()> {
        let cache = Cache::parse(CACHE_JSON)?;
        assert_eq!(cache.len(), 2);
        let totebag = cache.get("tamada", "totebag").expect("totebag is cached");
        assert_eq!(totebag.description.as_deref(), Some("archiver"));
        assert_eq!(totebag.releases.len(), 1);
        assert_eq!(totebag.languages, vec!["Rust".to_string()]);
        assert_eq!(
            totebag.last_updated.map(|t| t.to_rfc3339()),
            Some("2026-10-12T00:13:25+00:00".to_string())
        );
        // the products that are not in the cache are simply absent.
        assert!(cache.get("tamada", "pelicode").is_none());
        assert!(cache.get("someone-else", "totebag").is_none());
        Ok(())
    }

    #[test]
    fn empty_cache_holds_nothing() -> Result<()> {
        assert!(Cache::empty().is_empty());
        // the first build has an empty array published, which is a valid cache.
        assert!(Cache::parse("[]")?.is_empty());
        Ok(())
    }

    #[test]
    fn unreadable_entry_does_not_cost_the_whole_cache() -> Result<()> {
        let json = r#"[{"name":"no-owner"},
                       {"owner":"tamada","name":"totebag","last_updated":null}]"#;
        let cache = Cache::parse(json)?;
        assert_eq!(cache.len(), 1);
        assert!(cache.get("tamada", "totebag").is_some());
        Ok(())
    }

    #[test]
    fn a_broken_source_never_breaks_the_build() {
        // `load` reports the failure...
        assert!(Cache::load("does/not/exist.json").is_err());
        assert!(Cache::parse("not json at all").is_err());
        // ... while `load_or_empty` falls back to a full fetch.
        assert!(Cache::load_or_empty("does/not/exist.json").is_empty());
    }

    #[test]
    fn urls_and_paths_are_told_apart() {
        assert!(is_url("https://tamada.github.io/api/products/index.json"));
        assert!(is_url("http://localhost:8080/products/index.json"));
        assert!(!is_url("data/products.json"));
        assert!(!is_url("/tmp/products.json"));
    }
}
