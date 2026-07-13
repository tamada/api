use std::collections::{BTreeSet, HashMap};

use chrono::{DateTime, Utc};

use crate::github;
use crate::product::{build_product, InputItem, Product};

/// Updates all the given items and returns the list of full products.
///
/// First, this function fetches the last updated time (`updatedAt`) of all
/// the repositories of each owner at once (`queries/product_list.graphql`).
/// Then, it fetches the detail (`queries/products.graphql`) only for the
/// items that are base products or whose `last_updated` is older than the
/// repository's `updatedAt`.
///
/// A failure on a single item does not abort the whole process: the error is
/// logged and the previous information is kept as-is.
pub fn update_all(items: Vec<InputItem>) -> crate::Result<Vec<Product>> {
    github::is_auth_ok()?;
    let updated_map = fetch_updated_map(&items);
    Ok(items
        .into_iter()
        .map(|item| update_item(item, &updated_map))
        .collect())
}

type UpdatedMap = HashMap<String, HashMap<String, DateTime<Utc>>>;

fn fetch_updated_map(items: &[InputItem]) -> UpdatedMap {
    let owners = items
        .iter()
        .filter(|i| i.service() == "github")
        .map(|i| i.owner().to_string())
        .collect::<BTreeSet<_>>();
    let mut map = UpdatedMap::new();
    for owner in owners {
        match github::list_repositories(&owner) {
            Ok(m) => {
                log::info!("{}: {} repositories found", owner, m.len());
                map.insert(owner, m);
            }
            Err(e) => log::warn!("{}: failed to list repositories: {}", owner, e),
        }
    }
    map
}

fn update_item(item: InputItem, updated_map: &UpdatedMap) -> Product {
    let updated_at = updated_map
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
            match github::fetch_repository(&base.owner, &base.name) {
                Ok(detail) => build_product(
                    detail,
                    base.logo.clone(),
                    base.overrides.clone(),
                    HashMap::new(),
                    Vec::new(),
                    updated_at,
                ),
                Err(e) => {
                    log::error!("{}/{}: {}", base.owner, base.name, e);
                    base.to_product()
                }
            }
        }
        InputItem::Product(product) => {
            if is_up_to_date(&product, updated_at) {
                log::info!("{}/{}: up-to-date, skip", product.owner, product.name);
                return *product;
            }
            match github::fetch_repository(&product.owner, &product.name) {
                Ok(detail) => build_product(
                    detail,
                    product.logo.clone(),
                    product.overrides.clone(),
                    product.others.clone(),
                    product.releases.clone(),
                    updated_at,
                ),
                Err(e) => {
                    log::error!("{}/{}: {}", product.owner, product.name, e);
                    *product
                }
            }
        }
    }
}

fn is_up_to_date(product: &Product, updated_at: Option<DateTime<Utc>>) -> bool {
    match (product.last_updated, updated_at) {
        (Some(last), Some(updated)) => last >= updated,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::product::parse_items;

    #[test]
    fn up_to_date_check() {
        let json = r#"[{"owner":"tamada","name":"totebag","logo":null,
            "url":"https://tamada.github.io/totebag",
            "repository":"https://github.com/tamada/totebag",
            "sbom":"https://api.github.com/repos/tamada/totebag/dependency-graph/sbom",
            "last_updated":"2026-06-01T00:00:00Z"}]"#;
        let items = parse_items(json, false).unwrap();
        if let InputItem::Product(p) = &items[0] {
            let older = "2026-05-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
            let newer = "2026-07-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
            assert!(is_up_to_date(p, Some(older)));
            assert!(!is_up_to_date(p, Some(newer)));
            assert!(!is_up_to_date(p, None));
        } else {
            panic!("expected a full product");
        }
    }
}
