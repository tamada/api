use std::collections::HashMap;
use std::process::Command;

use chrono::{DateTime, Utc};
use rust_embed::Embed;
use serde::Deserialize;

use crate::{Error, Result};

#[derive(Embed)]
#[folder = "assets"]
struct Asset;

fn load_query(path: &str) -> Result<String> {
    match Asset::get(path) {
        Some(file) => match std::str::from_utf8(file.data.as_ref()) {
            Ok(s) => Ok(s.to_string()),
            Err(e) => Err(Error::Fatal(format!("{}: {}", path, e))),
        },
        None => Err(Error::Fatal(format!("{}: embedded query not found", path))),
    }
}

/// Checks whether the GitHub CLI is available and authenticated.
pub fn is_auth_ok() -> Result<()> {
    let output = match Command::new("gh").args(["auth", "status"]).output() {
        Ok(output) => output,
        Err(e) => return Err(Error::Io(e)),
    };
    if output.status.success() {
        Ok(())
    } else {
        let code = output.status.code().unwrap_or(-1);
        if code == 1 {
            Err(Error::GitHub(
                "GitHub CLI is not authenticated. Please run `gh auth login` to authenticate."
                    .into(),
            ))
        } else {
            Err(Error::GitHub(format!(
                "GitHub CLI authentication check failed with exit code: {}",
                code
            )))
        }
    }
}

fn run_gh(args: &[String]) -> Result<String> {
    log::info!("executing: gh {}", args.join(" "));
    let output = Command::new("gh").args(args).output().map_err(Error::Io)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(Error::GitHub(stderr))
    }
}

// ---------- list query (queries/product_list.graphql) ----------

#[derive(Deserialize)]
struct ListPage {
    data: ListData,
}

#[derive(Deserialize)]
struct ListData {
    user: ListUser,
}

#[derive(Deserialize)]
struct ListUser {
    repositories: ListRepositories,
}

#[derive(Deserialize)]
struct ListRepositories {
    nodes: Vec<Option<ListNode>>,
}

#[derive(Deserialize)]
struct ListNode {
    name: String,
    #[serde(rename = "updatedAt")]
    updated_at: DateTime<Utc>,
}

/// Fetches the map of the repository names to their last updated time
/// (`updatedAt`) of the given owner, by the paginated GraphQL query
/// `queries/product_list.graphql`.
pub fn list_repositories<S: AsRef<str>>(owner: S) -> Result<HashMap<String, DateTime<Utc>>> {
    let query = load_query("queries/product_list.graphql")?;
    let args = vec![
        "api".to_string(),
        "graphql".to_string(),
        "--paginate".to_string(),
        "--slurp".to_string(),
        "-F".to_string(),
        format!("owner={}", owner.as_ref()),
        "-f".to_string(),
        format!("query={}", query),
    ];
    let out = run_gh(&args)?;
    parse_list(&out)
}

fn parse_list(json: &str) -> Result<HashMap<String, DateTime<Utc>>> {
    let pages: Vec<ListPage> = serde_json::from_str(json).map_err(Error::Parse)?;
    Ok(pages
        .into_iter()
        .flat_map(|p| p.data.user.repositories.nodes)
        .flatten()
        .map(|n| (n.name, n.updated_at))
        .collect())
}

// ---------- detail query (queries/products.graphql) ----------

#[derive(Deserialize)]
struct DetailResponse {
    data: DetailData,
}

#[derive(Deserialize)]
struct DetailData {
    repository: RepoDetail,
}

#[derive(Deserialize, Debug)]
pub struct OwnerNode {
    pub login: String,
}

#[derive(Deserialize, Debug)]
pub struct NameNode {
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct LicenseNode {
    pub spdx: Option<String>,
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct LanguagesNode {
    pub nodes: Vec<NameNode>,
}

#[derive(Deserialize, Debug)]
pub struct ReleaseNode {
    pub name: Option<String>,
    #[serde(rename = "tagName")]
    pub tag_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "isDraft", default)]
    pub is_draft: bool,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "publishedAt", default)]
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Debug)]
pub struct TopicsNode {
    pub nodes: Vec<TopicNode>,
}

#[derive(Deserialize, Debug)]
pub struct TopicNode {
    pub topic: NameNode,
}

/// The repository detail obtained by `queries/products.graphql`.
#[derive(Deserialize, Debug)]
pub struct RepoDetail {
    pub name: String,
    pub owner: OwnerNode,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "homepageUrl", default)]
    pub homepage_url: Option<String>,
    pub url: String,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    /// `pushedAt` of the repository (aliased to `lastModifiedAt` in the query).
    #[serde(rename = "lastModifiedAt", default)]
    pub last_modified_at: Option<DateTime<Utc>>,
    #[serde(rename = "isArchived", default)]
    pub is_archived: bool,
    #[serde(rename = "licenseInfo", default)]
    pub license_info: Option<LicenseNode>,
    #[serde(default)]
    pub languages: Option<LanguagesNode>,
    #[serde(rename = "latestRelease", default)]
    pub latest_release: Option<ReleaseNode>,
    #[serde(rename = "repositoryTopics", default)]
    pub topics: Option<TopicsNode>,
}

/// Fetches the detail of the given repository by the GraphQL query
/// `queries/products.graphql`.
pub fn fetch_repository<S: AsRef<str>>(owner: S, name: S) -> Result<RepoDetail> {
    let query = load_query("queries/products.graphql")?;
    let args = vec![
        "api".to_string(),
        "graphql".to_string(),
        "-F".to_string(),
        format!("owner={}", owner.as_ref()),
        "-F".to_string(),
        format!("name={}", name.as_ref()),
        "-f".to_string(),
        format!("query={}", query),
    ];
    let out = run_gh(&args)?;
    parse_detail(&out)
}

fn parse_detail(json: &str) -> Result<RepoDetail> {
    serde_json::from_str::<DetailResponse>(json)
        .map(|r| r.data.repository)
        .map_err(Error::Parse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_list_pages() -> Result<()> {
        let json = r#"[{"data":{"user":{"repositories":{"totalCount":2,"nodes":[
            {"name":"fauxrest","description":null,"isArchived":false,"isPrivate":false,
             "createdAt":"2025-05-01T00:00:00Z","updatedAt":"2026-06-30T12:43:00Z"},
            {"name":"totebag","description":"archiver","isArchived":false,"isPrivate":false,
             "createdAt":"2024-01-01T00:00:00Z","updatedAt":"2026-01-15T08:00:00Z"}
        ],"pageInfo":{"endCursor":"abc","hasNextPage":false}}}}}]"#;
        let map = parse_list(json)?;
        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get("fauxrest").map(|d| d.to_rfc3339()),
            Some("2026-06-30T12:43:00+00:00".to_string())
        );
        Ok(())
    }

    #[test]
    fn parse_detail_response() -> Result<()> {
        let json = r#"{"data":{"repository":{
            "name":"totebag","owner":{"login":"tamada"},
            "description":"A tool for archiving/extracting files",
            "homepageUrl":"https://tamada.github.io/totebag",
            "url":"https://github.com/tamada/totebag",
            "createdAt":"2024-01-01T00:00:00Z",
            "lastModifiedAt":"2026-06-01T00:00:00Z",
            "isArchived":false,"isPrivate":false,"visibility":"PUBLIC","diskUsage":1234,
            "licenseInfo":{"spdx":"mit","name":"MIT License","url":"https://api.github.com/licenses/mit"},
            "primaryLanguage":{"name":"Rust","color":"\\#dea584"},
            "languages":{"nodes":[{"name":"Rust","color":"\\#dea584"},{"name":"Shell","color":null}]},
            "latestRelease":{"name":"Release v0.7.6","tagName":"v0.7.6",
              "url":"https://github.com/tamada/totebag/releases/tag/v0.7.6",
              "description":"bug fix","isDraft":false,
              "createdAt":"2026-05-30T00:00:00Z","publishedAt":"2026-06-01T00:00:00Z"},
            "repositoryTopics":{"nodes":[{"topic":{"name":"archive"},"url":"https://github.com/topics/archive"}]},
            "collaborators":{"totalCount":2},
            "stargazerCount":10,"forkCount":2,"watchers":{"totalCount":3}}}}"#;
        let detail = parse_detail(json)?;
        assert_eq!(detail.name, "totebag");
        assert_eq!(detail.owner.login, "tamada");
        assert_eq!(detail.license_info.as_ref().map(|l| l.spdx.as_deref()), Some(Some("mit")));
        assert_eq!(detail.languages.as_ref().map(|l| l.nodes.len()), Some(2));
        assert_eq!(detail.latest_release.as_ref().map(|r| r.tag_name.as_str()), Some("v0.7.6"));
        Ok(())
    }
}
