use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{Error, Result};
use crate::github::RepoDetail;

/// Keys that are real fields of [`Product`]. They are removed from the
/// flattened `others` map to avoid duplicated keys in the output JSON.
const PRODUCT_KEYS: [&str; 13] = [
    "owner", "name", "logo", "url", "repository", "description", "releases",
    "license", "languages", "sbom", "topics", "overrides", "last_updated",
];

/// Keys that must not be overridden by `overrides`, since they identify
/// the repository to be fetched.
const PROTECTED_KEYS: [&str; 2] = ["owner", "name"];

/// A release of a product (see `Release` in `assets/products.pkl`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Release {
    pub name: String,
    pub release_date: DateTime<Utc>,
    #[serde(default)]
    pub description: String,
}

/// A license of a product (see `License` in `assets/products.pkl`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct License {
    pub spdx: String,
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
}

/// The full product information (see `Product` in `assets/products.pkl`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub owner: String,
    pub name: String,
    #[serde(default)]
    pub logo: Option<String>,
    pub url: String,
    pub repository: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub releases: Vec<Release>,
    #[serde(default)]
    pub license: Vec<License>,
    #[serde(default)]
    pub languages: Vec<String>,
    pub sbom: String,
    #[serde(default)]
    pub topics: Vec<String>,
    /// The values overwriting the information fetched from GitHub
    /// (`overrides` of `ProductBase`). Kept in the output so that they are
    /// re-applied every time the product is refreshed.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub overrides: Map<String, Value>,
    /// `updatedAt` of the repository at the last fetch. Used to decide
    /// whether the product needs to be refreshed on the next run.
    #[serde(default)]
    pub last_updated: Option<DateTime<Utc>>,

    #[serde(flatten)]
    pub others: HashMap<String, Value>,
}

/// The base product information (see `ProductBase` in `assets/products.pkl`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseProduct {
    pub owner: String,
    pub name: String,
    #[serde(default)]
    pub logo: Option<String>,
    #[serde(default = "default_service")]
    pub service: String,
    /// The values overwriting the information fetched from GitHub.
    #[serde(default)]
    pub overrides: Map<String, Value>,
    #[serde(default)]
    pub last_updated: Option<DateTime<Utc>>,
}

fn default_service() -> String {
    "github".to_string()
}

/// An element of the input JSON array: either a full product or a base product.
#[derive(Debug, Clone)]
pub enum InputItem {
    Product(Box<Product>),
    Base(BaseProduct),
}

impl InputItem {
    pub fn owner(&self) -> &str {
        match self {
            InputItem::Product(p) => &p.owner,
            InputItem::Base(b) => &b.owner,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            InputItem::Product(p) => &p.name,
            InputItem::Base(b) => &b.name,
        }
    }

    pub fn service(&self) -> &str {
        match self {
            InputItem::Product(_) => "github",
            InputItem::Base(b) => &b.service,
        }
    }
}

/// Parses the input JSON text into a list of [`InputItem`]s.
/// Each element is recognized as a full [`Product`] when it contains
/// product-specific keys (`repository`, `sbom`, `releases`, ...), and as a
/// [`BaseProduct`] otherwise. If `force_base` is true, all the elements are
/// parsed as [`BaseProduct`]s.
pub fn parse_items(text: &str, force_base: bool) -> Result<Vec<InputItem>> {
    let values: Vec<Value> = serde_json::from_str(text).map_err(Error::Parse)?;
    let mut items = Vec::new();
    let mut errs = Vec::new();
    for value in values {
        match parse_item(value, force_base) {
            Ok(item) => items.push(item),
            Err(e) => errs.push(e),
        }
    }
    Error::from(items, errs)
}

fn parse_item(value: Value, force_base: bool) -> Result<InputItem> {
    if !force_base && is_full_product(&value) {
        serde_json::from_value::<Product>(value)
            .map(|mut p| {
                p.others.retain(|k, _| !PRODUCT_KEYS.contains(&k.as_str()));
                InputItem::Product(Box::new(p))
            })
            .map_err(Error::Parse)
    } else {
        serde_json::from_value::<BaseProduct>(value)
            .map(InputItem::Base)
            .map_err(Error::Parse)
    }
}

fn is_full_product(value: &Value) -> bool {
    ["repository", "sbom", "releases", "license", "languages", "topics", "url"]
        .iter()
        .any(|key| value.get(key).is_some())
}

impl BaseProduct {
    /// Builds a [`Product`] filled with the default values, without accessing GitHub.
    pub fn to_product(&self) -> Product {
        let product = Product {
            owner: self.owner.clone(),
            name: self.name.clone(),
            logo: self.logo.clone(),
            url: default_url(&self.owner, &self.name),
            repository: default_repository(&self.owner, &self.name),
            description: None,
            releases: Vec::new(),
            license: Vec::new(),
            languages: Vec::new(),
            sbom: default_sbom(&self.owner, &self.name),
            topics: Vec::new(),
            overrides: self.overrides.clone(),
            last_updated: self.last_updated,
            others: HashMap::new(),
        };
        apply_overrides(product)
    }
}

/// Overwrites the fields of the given product with the values in its
/// `overrides` map. The keys identifying the repository ([`PROTECTED_KEYS`])
/// and the `overrides` key itself are not overridable. When a value does not
/// fit the field type, a warning is logged and the product is left untouched.
pub fn apply_overrides(product: Product) -> Product {
    if product.overrides.is_empty() {
        return product;
    }
    let mut value = match serde_json::to_value(&product) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("{}/{}: failed to apply overrides: {}", product.owner, product.name, e);
            return product;
        }
    };
    if let Value::Object(map) = &mut value {
        for (key, v) in &product.overrides {
            if key == "overrides" || PROTECTED_KEYS.contains(&key.as_str()) {
                log::warn!(
                    "{}/{}: overrides.{} is not overridable, ignored",
                    product.owner, product.name, key
                );
                continue;
            }
            map.insert(key.clone(), v.clone());
        }
    }
    match serde_json::from_value::<Product>(value) {
        Ok(mut p) => {
            p.others.retain(|k, _| !PRODUCT_KEYS.contains(&k.as_str()));
            p
        }
        Err(e) => {
            log::warn!("{}/{}: failed to apply overrides: {}", product.owner, product.name, e);
            product
        }
    }
}

pub fn default_url(owner: &str, name: &str) -> String {
    format!("https://{}.github.io/{}", owner, name)
}

pub fn default_repository(owner: &str, name: &str) -> String {
    format!("https://github.com/{}/{}", owner, name)
}

pub fn default_sbom(owner: &str, name: &str) -> String {
    format!(
        "https://api.github.com/repos/{}/{}/dependency-graph/sbom",
        owner, name
    )
}

/// Builds a [`Product`] from the repository detail obtained from GitHub,
/// keeping the information (logo, overrides, known releases, and extra keys)
/// of the previous product or the base product, then applies `overrides`
/// on the fetched values.
pub fn build_product(
    detail: RepoDetail,
    logo: Option<String>,
    overrides: Map<String, Value>,
    others: HashMap<String, Value>,
    mut releases: Vec<Release>,
    last_updated: Option<DateTime<Utc>>,
) -> Product {
    let owner = detail.owner.login;
    let name = detail.name;
    if let Some(r) = detail.latest_release && !r.is_draft {
        let release_name = match r.name {
            Some(n) if !n.trim().is_empty() => n,
            _ => r.tag_name,
        };
        if !releases.iter().any(|e| e.name == release_name) {
            releases.push(Release {
                name: release_name,
                release_date: r.published_at.unwrap_or(r.created_at),
                description: r.description.unwrap_or_default(),
            });
        }
    }
    // releases.sort_by(|a, b| b.release_date.cmp(&a.release_date));
    releases.sort_by_key(|b| std::cmp::Reverse(b.release_date));    
    let license = detail
        .license_info
        .into_iter()
        .map(|l| License {
            spdx: l.spdx.unwrap_or_else(|| "unknown".to_string()),
            name: l.name,
            url: l.url,
        })
        .collect();
    let languages = detail
        .languages
        .map(|l| l.nodes.into_iter().map(|n| n.name).collect())
        .unwrap_or_default();
    let topics = detail
        .topics
        .map(|t| t.nodes.into_iter().map(|n| n.topic.name).collect())
        .unwrap_or_default();
    let url = match detail.homepage_url {
        Some(u) if !u.trim().is_empty() => u,
        _ => default_url(&owner, &name),
    };
    let sbom = default_sbom(&owner, &name);
    let repository = detail.url;
    let last_updated = last_updated.or(detail.last_modified_at).or_else(|| Some(Utc::now()));
    let product = Product {
        owner,
        name,
        logo,
        url,
        repository,
        description: detail.description,
        releases,
        license,
        languages,
        sbom,
        topics,
        overrides,
        last_updated,
        others,
    };
    apply_overrides(product)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE_JSON: &str = r#"[
        {"owner": "tamada", "name": "fauxrest", "logo": "https://example.com/logo.svg"},
        {"owner": "tamada", "name": "pick-a-boo", "service": "github",
         "overrides": {"url": "https://crates.io/crates/pick-a-boo"}}
    ]"#;

    const MIXED_JSON: &str = r#"[
        {"owner": "tamada", "name": "fauxrest", "logo": null},
        {"owner": "tamada", "name": "totebag", "logo": null,
         "url": "https://tamada.github.io/totebag",
         "repository": "https://github.com/tamada/totebag",
         "releases": [{"name": "v0.7.5", "release_date": "2025-01-01T00:00:00Z", "description": ""}],
         "license": [{"spdx": "mit", "name": "MIT License", "url": null}],
         "languages": ["Rust"],
         "sbom": "https://api.github.com/repos/tamada/totebag/dependency-graph/sbom",
         "topics": ["archive"],
         "last_updated": "2025-06-01T00:00:00Z"}
    ]"#;

    #[test]
    fn parse_base_products() -> Result<()> {
        let items = parse_items(BASE_JSON, false)?;
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], InputItem::Base(_)));
        if let InputItem::Base(b) = &items[1] {
            assert_eq!(b.service, "github");
            assert_eq!(
                b.overrides.get("url").and_then(|v| v.as_str()),
                Some("https://crates.io/crates/pick-a-boo")
            );
        }
        Ok(())
    }

    #[test]
    fn parse_mixed_products() -> Result<()> {
        let items = parse_items(MIXED_JSON, false)?;
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], InputItem::Base(_)));
        match &items[1] {
            InputItem::Product(p) => {
                assert_eq!(p.name, "totebag");
                assert_eq!(p.releases.len(), 1);
                assert_eq!(p.license[0].spdx, "mit");
                assert!(p.last_updated.is_some());
            }
            _ => panic!("expected a full product"),
        }
        Ok(())
    }

    #[test]
    fn parse_forced_base() -> Result<()> {
        let items = parse_items(BASE_JSON, true)?;
        assert!(items.iter().all(|i| matches!(i, InputItem::Base(_))));
        Ok(())
    }

    #[test]
    fn base_to_product_defaults() {
        let items = parse_items(BASE_JSON, false).unwrap();
        if let InputItem::Base(b) = &items[0] {
            let p = b.to_product();
            assert_eq!(p.url, "https://tamada.github.io/fauxrest");
            assert_eq!(p.repository, "https://github.com/tamada/fauxrest");
            assert_eq!(
                p.sbom,
                "https://api.github.com/repos/tamada/fauxrest/dependency-graph/sbom"
            );
        }
    }

    #[test]
    fn overrides_overwrite_fetched_values() {
        let items = parse_items(BASE_JSON, false).unwrap();
        if let InputItem::Base(b) = &items[1] {
            // to_product applies overrides: url is overwritten by crates.io.
            let p = b.to_product();
            assert_eq!(p.url, "https://crates.io/crates/pick-a-boo");
            // overrides themselves are kept for the next refresh.
            assert_eq!(
                p.overrides.get("url").and_then(|v| v.as_str()),
                Some("https://crates.io/crates/pick-a-boo")
            );
        } else {
            panic!("expected a base product");
        }
    }

    #[test]
    fn overrides_ignore_protected_keys() {
        let json = r#"[{"owner": "tamada", "name": "fauxrest",
            "overrides": {"name": "renamed", "description": "overridden"}}]"#;
        let items = parse_items(json, false).unwrap();
        if let InputItem::Base(b) = &items[0] {
            let p = b.to_product();
            assert_eq!(p.name, "fauxrest");
            assert_eq!(p.description.as_deref(), Some("overridden"));
        } else {
            panic!("expected a base product");
        }
    }
}
