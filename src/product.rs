use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{Error, Result};
use crate::github::RepoDetail;

/// Keys that are real fields of [`Product`]. They are removed from the
/// flattened `others` map to avoid duplicated keys in the output JSON.
const PRODUCT_KEYS: [&str; 10] = [
    "owner", "name", "description", "releases", "license", "links",
    "languages", "topics", "overrides", "last_updated",
];

/// Keys that used to be fields of [`Product`]. They live in `links` now, so
/// they are dropped from the flattened `others` map: otherwise an `overrides`
/// or an old input JSON carrying them would bring them back to the output.
const RETIRED_KEYS: [&str; 4] = ["logo", "url", "repository", "sbom"];

/// Keys that must not be overridden by `overrides`, since they identify
/// the repository to be fetched.
const PROTECTED_KEYS: [&str; 2] = ["owner", "name"];

/// Drops the keys that are already a field of [`Product`], and the ones that
/// have been retired in favor of `links`.
fn retain_others(others: &mut HashMap<String, Value>) {
    others.retain(|k, _| {
        !PRODUCT_KEYS.contains(&k.as_str()) && !RETIRED_KEYS.contains(&k.as_str())
    });
}

/// A release of a product (see `Release` in `assets/base-products.pkl`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Release {
    pub name: String,
    pub release_date: DateTime<Utc>,
    #[serde(default)]
    pub description: String,
}

/// A license of a product (see `License` in `assets/base-products.pkl`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct License {
    pub spdx: String,
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
}

/// A link of a product (see `Link` in `assets/base-products.pkl`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Link {
    pub label: String,
    pub url: String,
}

/// An element of `links` in the input JSON: either a full [`Link`], or a bare
/// link type (`"sbom"`, `"repository"`, ...) whose URL is derived from the
/// `owner` and `name` of the product (see `LinkType` in
/// `assets/base-products.pkl`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum LinkSpec {
    Type(String),
    Link(Link),
}

/// The full product information (see `Product` in `assets/base-products.pkl`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub owner: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub releases: Vec<Release>,
    #[serde(default)]
    pub license: Vec<License>,
    /// Every URL of the product in a single array: the logo, the web site, the
    /// repository, the SBOM, and so on. The bare link types of the input
    /// (`ProductBase.links`) are resolved into concrete URLs here.
    #[serde(default)]
    pub links: Vec<Link>,
    #[serde(default)]
    pub languages: Vec<String>,
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

/// The base product information (see `ProductBase` in `assets/base-products.pkl`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseProduct {
    pub owner: String,
    pub name: String,
    /// The logo given as its own key instead of a `logo` link. It is turned
    /// into a link on the output, since the output has no `logo` field.
    #[serde(default)]
    pub logo: Option<String>,
    #[serde(default = "default_service")]
    pub service: String,
    /// The values overwriting the information fetched from GitHub.
    #[serde(default)]
    pub overrides: Map<String, Value>,
    #[serde(default)]
    pub last_updated: Option<DateTime<Utc>>,
    #[serde(default)]
    pub links: Vec<LinkSpec>,
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
/// product-specific keys (`releases`, `license`, `languages`, ...), and as a
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
                retain_others(&mut p.others);
                InputItem::Product(Box::new(p))
            })
            .map_err(Error::Parse)
    } else {
        serde_json::from_value::<BaseProduct>(value)
            .map(InputItem::Base)
            .map_err(Error::Parse)
    }
}

/// `links` is not usable to tell the two apart, since a base product has it too.
fn is_full_product(value: &Value) -> bool {
    ["description", "releases", "license", "languages", "topics"]
        .iter()
        .any(|key| value.get(key).is_some())
}

/// The URLs of the repository fetched from GitHub. They are preferred to the
/// URLs derived from `owner` and `name` when resolving the bare link types.
/// The default (every field `None`) is used when GitHub is not accessed.
#[derive(Debug, Default)]
struct FetchedUrls {
    /// `homepageUrl` of the repository.
    www: Option<String>,
    /// `url` of the repository.
    repository: Option<String>,
}

impl FetchedUrls {
    fn of(detail: &RepoDetail) -> Self {
        Self {
            www: detail
                .homepage_url
                .as_ref()
                .map(|u| u.trim())
                .filter(|u| !u.is_empty())
                .map(std::string::ToString::to_string),
            repository: Some(detail.url.clone()),
        }
    }
}

/// Resolves the bare link types of the input into concrete [`Link`]s, keeping
/// the links given with their own URL as they are. A bare `www` and
/// `repository` take the value fetched from GitHub, falling back to the URL
/// derived from `owner` and `name`; a bare `sbom` is always derived, since
/// GitHub does not report it. The types that cannot be derived (`logo`,
/// `docs`, `registry`, `container`) must be given as a link with their own
/// URL; otherwise they are dropped with a warning.
fn resolve_links(
    specs: &[LinkSpec],
    owner: &str,
    name: &str,
    fetched: &FetchedUrls,
) -> Vec<Link> {
    specs
        .iter()
        .filter_map(|spec| match spec {
            LinkSpec::Link(link) => Some(link.clone()),
            LinkSpec::Type(label) => match derive_link_url(label, owner, name, fetched) {
                Some(url) => Some(Link { label: label.clone(), url }),
                None => {
                    log::warn!(
                        "{}/{}: links.{} has no URL to derive, ignored",
                        owner, name, label
                    );
                    None
                }
            },
        })
        .collect()
}

fn derive_link_url(
    label: &str,
    owner: &str,
    name: &str,
    fetched: &FetchedUrls,
) -> Option<String> {
    match label {
        "www" => Some(
            fetched.www.clone().unwrap_or_else(|| default_url(owner, name)),
        ),
        "repository" => Some(
            fetched.repository.clone().unwrap_or_else(|| default_repository(owner, name)),
        ),
        "sbom" => Some(default_sbom(owner, name)),
        _ => None,
    }
}

/// Builds the link specs of a base product, turning its `logo` key into a
/// `logo` link. The key is ignored when the links already have their own.
fn link_specs(logo: &Option<String>, links: &[LinkSpec]) -> Vec<LinkSpec> {
    let has_logo = links.iter().any(|spec| match spec {
        LinkSpec::Link(link) => link.label == "logo",
        LinkSpec::Type(label) => label == "logo",
    });
    match logo {
        Some(url) if !has_logo => {
            let logo = LinkSpec::Link(Link { label: "logo".to_string(), url: url.clone() });
            std::iter::once(logo).chain(links.iter().cloned()).collect()
        }
        _ => links.to_vec(),
    }
}

impl BaseProduct {
    /// Builds a [`Product`] filled with the default values, without accessing GitHub.
    pub fn to_product(&self) -> Product {
        let product = Product {
            owner: self.owner.clone(),
            name: self.name.clone(),
            description: None,
            releases: Vec::new(),
            license: Vec::new(),
            links: resolve_links(
                &link_specs(&self.logo, &self.links),
                &self.owner,
                &self.name,
                &FetchedUrls::default(),
            ),
            languages: Vec::new(),
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
            retain_others(&mut p.others);
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
/// keeping the information (links, overrides, known releases, and extra keys)
/// of the previous product or the base product, then applies `overrides`
/// on the fetched values.
pub fn build_product(
    detail: RepoDetail,
    logo: Option<String>,
    overrides: Map<String, Value>,
    others: HashMap<String, Value>,
    mut releases: Vec<Release>,
    last_updated: Option<DateTime<Utc>>,
    links: &[LinkSpec],
) -> Product {
    let fetched = FetchedUrls::of(&detail);
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
    let last_updated = last_updated.or(detail.last_modified_at).or_else(|| Some(Utc::now()));
    let links = resolve_links(&link_specs(&logo, links), &owner, &name, &fetched);
    let product = Product {
        owner,
        name,
        description: detail.description,
        releases,
        license,
        links,
        languages,
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
        {"owner": "tamada", "name": "totebag",
         "releases": [{"name": "v0.7.5", "release_date": "2025-01-01T00:00:00Z", "description": ""}],
         "license": [{"spdx": "mit", "name": "MIT License", "url": null}],
         "links": [{"label": "www", "url": "https://tamada.github.io/totebag"},
                   {"label": "repository", "url": "https://github.com/tamada/totebag"}],
         "languages": ["Rust"],
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
                assert_eq!(p.links.len(), 2);
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
        let json = r#"[{"owner": "tamada", "name": "fauxrest",
            "logo": "https://example.com/logo.svg",
            "links": ["www", "repository", "sbom"]}]"#;
        let items = parse_items(json, false).unwrap();
        if let InputItem::Base(b) = &items[0] {
            let p = b.to_product();
            let links = p.links.iter()
                .map(|l| (l.label.as_str(), l.url.as_str()))
                .collect::<Vec<_>>();
            // the `logo` key of the base product becomes the first link, and
            // the bare types are derived from the owner and the name.
            assert_eq!(links, vec![
                ("logo", "https://example.com/logo.svg"),
                ("www", "https://tamada.github.io/fauxrest"),
                ("repository", "https://github.com/tamada/fauxrest"),
                ("sbom", "https://api.github.com/repos/tamada/fauxrest/dependency-graph/sbom"),
            ]);
        } else {
            panic!("expected a base product");
        }
    }

    #[test]
    fn retired_keys_never_reach_the_output() {
        let items = parse_items(BASE_JSON, false).unwrap();
        if let InputItem::Base(b) = &items[1] {
            let p = b.to_product();
            // `url` is not a field anymore, hence the override cannot bring it
            // back through the flattened extra keys.
            assert!(!p.others.contains_key("url"));
            // overrides themselves are kept for the next refresh.
            assert_eq!(
                p.overrides.get("url").and_then(|v| v.as_str()),
                Some("https://crates.io/crates/pick-a-boo")
            );
        } else {
            panic!("expected a base product");
        }
    }

    const LINKS_JSON: &str = r#"[
        {"owner": "tamada", "name": "fauxrest",
         "links": [{"label": "logo", "url": "https://example.com/fauxrest.svg"},
                   {"label": "registry", "url": "https://crates.io/crates/fauxrest"},
                   "sbom", "repository", "www", "docs"]}
    ]"#;

    #[test]
    fn resolve_links_of_base_product() {
        let items = parse_items(LINKS_JSON, false).unwrap();
        if let InputItem::Base(b) = &items[0] {
            let p = b.to_product();
            let labels = p.links.iter().map(|l| l.label.as_str()).collect::<Vec<_>>();
            // `docs` has no URL to derive, hence it is dropped.
            assert_eq!(labels, vec!["logo", "registry", "sbom", "repository", "www"]);
            assert_eq!(
                p.links[2].url,
                "https://api.github.com/repos/tamada/fauxrest/dependency-graph/sbom"
            );
            assert_eq!(p.links[3].url, "https://github.com/tamada/fauxrest");
            assert_eq!(p.links[4].url, "https://tamada.github.io/fauxrest");
        } else {
            panic!("expected a base product");
        }
    }

    #[test]
    fn overrides_replace_the_whole_links() {
        let json = r#"[{"owner": "tamada", "name": "pick-a-boo",
            "overrides": {"links": [{"label": "registry", "url": "https://crates.io/crates/pick-a-boo"}]},
            "links": ["www", "repository"]}]"#;
        let items = parse_items(json, false).unwrap();
        if let InputItem::Base(b) = &items[0] {
            let p = b.to_product();
            assert_eq!(p.links.len(), 1);
            assert_eq!(p.links[0].label, "registry");
            assert_eq!(p.links[0].url, "https://crates.io/crates/pick-a-boo");
        } else {
            panic!("expected a base product");
        }
    }

    /// Builds a [`RepoDetail`] carrying the URLs reported by GitHub.
    fn repo_detail(homepage_url: &str) -> RepoDetail {
        let json = format!(
            r#"{{"name":"totebag","owner":{{"login":"tamada"}},
                "homepageUrl":{},
                "url":"https://github.com/tamada/totebag-renamed",
                "createdAt":"2024-01-01T00:00:00Z"}}"#,
            serde_json::to_string(homepage_url).unwrap()
        );
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn fetched_urls_win_over_the_derived_ones() {
        let links = vec![
            LinkSpec::Type("www".to_string()),
            LinkSpec::Type("repository".to_string()),
            LinkSpec::Type("sbom".to_string()),
        ];
        let p = build_product(
            repo_detail("https://totebag.example.com"),
            None,
            Map::new(),
            HashMap::new(),
            Vec::new(),
            None,
            &links,
        );
        assert_eq!(p.links[0].url, "https://totebag.example.com");
        // the repository may have been renamed, so the fetched URL is used.
        assert_eq!(p.links[1].url, "https://github.com/tamada/totebag-renamed");
        // GitHub does not report the SBOM URL, hence it stays derived.
        assert_eq!(
            p.links[2].url,
            "https://api.github.com/repos/tamada/totebag/dependency-graph/sbom"
        );
    }

    #[test]
    fn blank_homepage_falls_back_to_the_derived_url() {
        let links = vec![LinkSpec::Type("www".to_string())];
        let p = build_product(
            repo_detail("   "),
            None,
            Map::new(),
            HashMap::new(),
            Vec::new(),
            None,
            &links,
        );
        assert_eq!(p.links[0].url, "https://tamada.github.io/totebag");
    }

    #[test]
    fn explicit_link_wins_over_the_fetched_url() {
        let links = vec![LinkSpec::Link(Link {
            label: "www".to_string(),
            url: "https://crates.io/crates/totebag".to_string(),
        })];
        let p = build_product(
            repo_detail("https://totebag.example.com"),
            None,
            Map::new(),
            HashMap::new(),
            Vec::new(),
            None,
            &links,
        );
        assert_eq!(p.links[0].url, "https://crates.io/crates/totebag");
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
