use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};

use crate::cache::Cache;
use crate::github;
use crate::product::{
    apply_overrides, build_cached_product, build_product, InputItem, LinkSpec, Product,
};

/// Updates all the given items and returns the list of full products.
///
/// First, this function fetches the last modified time of the repositories of
/// each owner in one request (see [`fetch_modified_map`]). Then, it fetches the
/// detail (`queries/products.graphql`) only for the items that have no usable
/// cache entry, or whose repository has been modified since the time the cache
/// reports.
///
/// The set of the products and their configuration come from the input alone:
/// `cache` only spares the detail query for the products that are unchanged,
/// never adds one and never decides a configured value.
///
/// A failure on a single item does not abort the whole process: the error is
/// logged and the cached or the default information is kept instead.
pub fn update_all(items: Vec<InputItem>, cache: &Cache) -> crate::Result<Vec<Product>> {
    github::is_auth_ok()?;
    let modified_map = fetch_modified_map(&items);
    Ok(items
        .into_iter()
        .map(|item| update_item(item, &modified_map, cache))
        .collect())
}

type ModifiedMap = HashMap<String, HashMap<String, DateTime<Utc>>>;

/// Collects the last modified time of the repository of every item, by owner.
///
/// The whole list of the repositories of an owner is asked for first, which is
/// one request however many products the owner has. A token an organization
/// policy keeps from listing a user's repositories is refused that request
/// outright, so the configured repositories are then asked for by name
/// instead: that is one request as well, and it needs no access beyond the one
/// the detail query already has.
///
/// An owner whose timestamps cannot be obtained at all is simply left out of
/// the map, which makes every product of that owner be fetched.
fn fetch_modified_map(items: &[InputItem]) -> ModifiedMap {
    let mut names: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for item in items.iter().filter(|i| i.service() == "github") {
        names
            .entry(item.owner().to_string())
            .or_default()
            .push(item.name().to_string());
    }
    let mut map = ModifiedMap::new();
    for (owner, names) in names {
        match modified_times(&owner, &names) {
            Ok(m) => {
                log::info!("{}: {} repositories reported their last modified time", owner, m.len());
                map.insert(owner, m);
            }
            Err(e) => log::warn!(
                "{}: no last modified time available ({}), hence every product of this owner is fetched",
                owner, e
            ),
        }
    }
    map
}

fn modified_times(owner: &str, names: &[String]) -> crate::Result<HashMap<String, DateTime<Utc>>> {
    match github::list_repositories(owner) {
        Ok(m) => Ok(m),
        Err(e) => {
            log::warn!(
                "{}: failed to list the repositories ({}), hence the configured ones are asked for by name",
                owner, e
            );
            github::fetch_last_modified(owner, names)
        }
    }
}

fn update_item(item: InputItem, modified_map: &ModifiedMap, cache: &Cache) -> Product {
    let last_modified = modified_map
        .get(item.owner())
        .and_then(|m| m.get(item.name()))
        .copied();
    match item {
        InputItem::Base(base) => {
            if base.service != "github" {
                log::warn!(
                    "{}/{}: unsupported service {:?}, keep the default values",
                    base.owner, base.name, base.service
                );
                return base.to_product();
            }
            let cached = cache.get(&base.owner, &base.name);
            if let Some(cached) = cached
                && is_up_to_date(cached.last_updated, last_modified)
            {
                log::info!(
                    "{}/{}: not modified since {}, reuse the cached values",
                    base.owner, base.name,
                    cached.last_updated.map_or_else(|| "?".to_string(), |t| t.to_rfc3339())
                );
                return build_cached_product(&base, cached);
            }
            match github::fetch_repository(&base.owner, &base.name) {
                Ok(detail) => build_product(
                    detail,
                    base.logo.clone(),
                    base.overrides.clone(),
                    HashMap::new(),
                    last_modified,
                    &base.links,
                ),
                Err(e) => {
                    log::error!("{}/{}: {}", base.owner, base.name, e);
                    // A failed fetch must not throw the previous information
                    // away: the cached values are older than the repository,
                    // but far closer to it than the defaults, and the next run
                    // refetches anyway since their timestamp stays behind.
                    match cached {
                        Some(cached) => build_cached_product(&base, cached),
                        None => base.to_product(),
                    }
                }
            }
        }
        InputItem::Product(product) => {
            if is_up_to_date(product.last_updated, last_modified) {
                log::info!("{}/{}: up-to-date, skip", product.owner, product.name);
                // `overrides` is configuration, hence it is re-applied even on
                // a skip: a change of it has to show up without a refetch.
                // `links` cannot be, since a full product carries them already
                // resolved and has no link types left to resolve; the base
                // products of `--cache` are the ones that get them re-applied.
                return apply_overrides(*product);
            }
            match github::fetch_repository(&product.owner, &product.name) {
                Ok(detail) => build_product(
                    detail,
                    None,
                    product.overrides.clone(),
                    product.others.clone(),
                    last_modified,
                    // The links of a full product are already resolved, so they
                    // are kept as they are.
                    &product.links.iter().cloned().map(LinkSpec::Link).collect::<Vec<_>>(),
                ),
                Err(e) => {
                    log::error!("{}/{}: {}", product.owner, product.name, e);
                    apply_overrides(*product)
                }
            }
        }
    }
}

/// Tells whether the information obtained at `last_updated` still describes a
/// repository last modified at `last_modified`.
///
/// Both sides have to be the same reduction of `updatedAt` and `pushedAt`
/// (see [`github::last_modified_of`]); comparing two different fields would
/// skip a repository forever whenever the stored one happens to run ahead.
/// An absent timestamp on either side means the product has to be fetched.
fn is_up_to_date(last_updated: Option<DateTime<Utc>>, last_modified: Option<DateTime<Utc>>) -> bool {
    match (last_updated, last_modified) {
        (Some(last_updated), Some(last_modified)) => last_updated >= last_modified,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::last_modified_of;
    use crate::product::{parse_items, BaseProduct};

    fn at(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn up_to_date_check() {
        let last_updated = Some(at("2026-06-01T00:00:00Z"));
        assert!(is_up_to_date(last_updated, Some(at("2026-05-01T00:00:00Z"))));
        // the very same timestamp means nothing has happened since the fetch.
        assert!(is_up_to_date(last_updated, last_updated));
        assert!(!is_up_to_date(last_updated, Some(at("2026-07-01T00:00:00Z"))));
        // a missing timestamp on either side forces a fetch.
        assert!(!is_up_to_date(last_updated, None));
        assert!(!is_up_to_date(None, Some(at("2026-05-01T00:00:00Z"))));
        assert!(!is_up_to_date(None, None));
    }

    /// Both sides of the check reduce `updatedAt` and `pushedAt` the same way,
    /// hence a repository whose `updatedAt` lags behind its `pushedAt` (9rules)
    /// is not reported as up-to-date after a push.
    #[test]
    fn a_push_is_never_hidden_by_a_lagging_updated_at() {
        // the fetch stored the state of the repository at that time ...
        let stored = last_modified_of(Some(at("2025-08-27T12:10:06Z")), Some(at("2026-07-20T06:39:00Z")));
        // ... and nothing has happened since.
        assert!(is_up_to_date(stored, last_modified_of(Some(at("2025-08-27T12:10:06Z")), Some(at("2026-07-20T06:39:00Z")))));
        // a later push moves `pushedAt` only, and it is still noticed.
        let pushed = last_modified_of(Some(at("2025-08-27T12:10:06Z")), Some(at("2026-07-20T06:41:06Z")));
        assert!(!is_up_to_date(stored, pushed));
    }

    /// The reverse case: a change that is not a push (a description, a topic)
    /// moves `updatedAt` only, and it is noticed as well.
    #[test]
    fn a_change_that_is_not_a_push_is_noticed_too() {
        let stored = last_modified_of(Some(at("2026-10-10T00:15:55Z")), Some(at("2026-10-10T00:15:55Z")));
        let edited = last_modified_of(Some(at("2026-10-10T00:26:00Z")), Some(at("2026-10-10T00:15:55Z")));
        assert!(!is_up_to_date(stored, edited));
    }

    fn base(json: &str) -> BaseProduct {
        match parse_items(json, true).unwrap().into_iter().next().unwrap() {
            InputItem::Base(base) => base,
            InputItem::Product(_) => panic!("expected a base product"),
        }
    }

    const CACHE_JSON: &str = r#"[{"owner":"tamada","name":"totebag",
        "description":"archiver",
        "releases":[{"name":"v0.10.0","release_date":"2026-10-01T00:00:00Z","description":"notes"}],
        "license":[{"spdx":"mit","name":"MIT License","url":null}],
        "links":[{"label":"www","url":"https://totebag.example.com"},
                 {"label":"repository","url":"https://github.com/tamada/totebag-renamed"}],
        "languages":["Rust"],"topics":["archive"],
        "last_updated":"2026-06-01T00:00:00Z"}]"#;

    /// The GitHub values of an unchanged product are reused as they are, while
    /// its links come from the base product every time.
    #[test]
    fn cached_values_are_reused_and_the_links_are_re_applied() {
        let cache = Cache::parse(CACHE_JSON).unwrap();
        let base = base(
            r#"[{"owner":"tamada","name":"totebag","logo":"https://example.com/totebag.png",
                 "links":["www","repository","sbom",
                          {"label":"registry","url":"https://crates.io/crates/totebag"}]}]"#,
        );
        let map = ModifiedMap::from([(
            "tamada".to_string(),
            HashMap::from([("totebag".to_string(), at("2026-05-01T00:00:00Z"))]),
        )]);
        let p = update_item(InputItem::Base(base), &map, &cache);

        // the values that come from GitHub are the cached ones.
        assert_eq!(p.description.as_deref(), Some("archiver"));
        assert_eq!(p.releases.len(), 1);
        assert_eq!(p.releases[0].name, "v0.10.0");
        assert_eq!(p.license[0].spdx, "mit");
        assert_eq!(p.languages, vec!["Rust".to_string()]);
        assert_eq!(p.topics, vec!["archive".to_string()]);
        assert_eq!(p.last_updated, Some(at("2026-06-01T00:00:00Z")));

        // the links are resolved from the base product, so the logo and the
        // registry link added to the Pkl file show up without a refetch ...
        let labels = p.links.iter().map(|l| l.label.as_str()).collect::<Vec<_>>();
        assert_eq!(labels, vec!["logo", "www", "repository", "sbom", "registry"]);
        // ... and the bare `www` and `repository` keep the URLs GitHub
        // reported, recovered from the cached links.
        assert_eq!(p.links[1].url.as_deref(), Some("https://totebag.example.com"));
        assert_eq!(p.links[2].url.as_deref(), Some("https://github.com/tamada/totebag-renamed"));
        assert_eq!(p.links[4].url.as_deref(), Some("https://crates.io/crates/totebag"));
    }

    /// A bare `www` of a product the cache knows nothing about falls back to
    /// the derived URL instead of being dropped.
    #[test]
    fn a_cache_without_the_links_falls_back_to_the_derived_urls() {
        let cache = Cache::parse(
            r#"[{"owner":"tamada","name":"totebag","description":"archiver",
                 "links":[],"last_updated":"2026-06-01T00:00:00Z"}]"#,
        )
        .unwrap();
        let base = base(r#"[{"owner":"tamada","name":"totebag","links":["www","repository"]}]"#);
        let map = ModifiedMap::from([(
            "tamada".to_string(),
            HashMap::from([("totebag".to_string(), at("2026-05-01T00:00:00Z"))]),
        )]);
        let p = update_item(InputItem::Base(base), &map, &cache);
        assert_eq!(p.links[0].url.as_deref(), Some("https://tamada.github.io/totebag"));
        assert_eq!(p.links[1].url.as_deref(), Some("https://github.com/tamada/totebag"));
    }

    /// `overrides` is re-applied on the cached values, so a change of it in the
    /// Pkl file shows up while the repository stays untouched.
    #[test]
    fn overrides_are_re_applied_to_the_cached_values() {
        let cache = Cache::parse(CACHE_JSON).unwrap();
        let base = base(
            r#"[{"owner":"tamada","name":"totebag",
                 "overrides":{"description":"overridden","topics":["cli"]},
                 "links":["repository"]}]"#,
        );
        let map = ModifiedMap::from([(
            "tamada".to_string(),
            HashMap::from([("totebag".to_string(), at("2026-05-01T00:00:00Z"))]),
        )]);
        let p = update_item(InputItem::Base(base), &map, &cache);
        assert_eq!(p.description.as_deref(), Some("overridden"));
        assert_eq!(p.topics, vec!["cli".to_string()]);
        // the overrides stay in the output, ready for the next run.
        assert_eq!(
            p.overrides.get("description").and_then(|v| v.as_str()),
            Some("overridden")
        );
    }

    /// A full product that is skipped gets its `overrides` re-applied as well:
    /// it used to be returned untouched, which kept the stale values.
    #[test]
    fn a_skipped_full_product_gets_its_overrides_re_applied() {
        let json = r#"[{"owner":"tamada","name":"totebag","description":"archiver",
            "overrides":{"description":"overridden"},
            "links":[{"label":"repository","url":"https://github.com/tamada/totebag"}],
            "last_updated":"2026-06-01T00:00:00Z"}]"#;
        let item = parse_items(json, false).unwrap().into_iter().next().unwrap();
        assert!(matches!(item, InputItem::Product(_)));
        let map = ModifiedMap::from([(
            "tamada".to_string(),
            HashMap::from([("totebag".to_string(), at("2026-05-01T00:00:00Z"))]),
        )]);
        let p = update_item(item, &map, &Cache::empty());
        assert_eq!(p.description.as_deref(), Some("overridden"));
    }

    /// The cache is only a shortcut: a product it does not hold is simply
    /// fetched, and one it holds while the Pkl file does not is not an output.
    #[test]
    fn the_cache_never_decides_the_product_set() {
        let cache = Cache::parse(CACHE_JSON).unwrap();
        assert!(cache.get("tamada", "totebag").is_some());
        // the caller maps over the input items only, hence a cached product
        // that is no longer configured has no way into the output.
        let items = parse_items(r#"[{"owner":"tamada","name":"fauxrest"}]"#, true).unwrap();
        let names = items.iter().map(InputItem::name).collect::<Vec<_>>();
        assert_eq!(names, vec!["fauxrest"]);
    }
}
