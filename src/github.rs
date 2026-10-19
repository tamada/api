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
    let (ok, stdout, stderr) = run_gh_output(args)?;
    if ok {
        Ok(stdout)
    } else {
        Err(Error::GitHub(stderr))
    }
}

/// Runs `gh` and hands back its output whether it succeeded or not, since
/// `gh api graphql` reports a response that carries both data and errors as a
/// failure while still printing the data it did get.
fn run_gh_output(args: &[String]) -> Result<(bool, String, String)> {
    log::info!("executing: gh {}", args.join(" "));
    let output = Command::new("gh").args(args).output().map_err(Error::Io)?;
    Ok((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
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
    #[serde(rename = "updatedAt", default)]
    updated_at: Option<DateTime<Utc>>,
    #[serde(rename = "pushedAt", default)]
    pushed_at: Option<DateTime<Utc>>,
}

/// The later of the two timestamps, ignoring the absent ones.
///
/// `updatedAt` and `pushedAt` move independently: `updatedAt` also follows the
/// changes that are not a push (the description, the topics, ...), while
/// `pushedAt` is the only one that follows a push to a non-default branch.
/// Neither is therefore above the other, and a repository is considered
/// modified at the later of the two. Both the listing and the detail reduce
/// their timestamps this way, so that the two sides of the up-to-date check
/// compare the very same value.
pub fn last_modified_of(
    updated_at: Option<DateTime<Utc>>,
    pushed_at: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    match (updated_at, pushed_at) {
        (Some(updated), Some(pushed)) => Some(updated.max(pushed)),
        (updated, pushed) => updated.or(pushed),
    }
}

/// Fetches the map of the repository names to their last modified time
/// (see [`last_modified_of`]) of the given owner, by the paginated GraphQL
/// query `queries/product_list.graphql`.
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
        .filter_map(|n| last_modified_of(n.updated_at, n.pushed_at).map(|t| (n.name, t)))
        .collect())
}

// ---------- timestamps of the named repositories ----------

#[derive(Deserialize)]
struct NamedPage {
    #[serde(default)]
    data: HashMap<String, Option<ListNode>>,
}

/// Fetches the last modified time (see [`last_modified_of`]) of the named
/// repositories of the given owner, in a single GraphQL request that aliases
/// one `repository` field per name.
///
/// This is the fallback for [`list_repositories`], whose `user(login:)` query
/// asks about every repository the token can see: an organization policy the
/// token does not satisfy refuses such a request outright, even though the
/// products themselves are public and their detail is fetched without trouble.
/// Asking for the configured repositories by name touches nothing else, hence
/// it needs the very same access the detail query already has.
///
/// A name that cannot be read (a renamed or a deleted repository) only leaves
/// its own entry out of the map, which makes that product be fetched.
pub fn fetch_last_modified(
    owner: &str,
    names: &[String],
) -> Result<HashMap<String, DateTime<Utc>>> {
    if names.is_empty() {
        return Ok(HashMap::new());
    }
    let args = vec![
        "api".to_string(),
        "graphql".to_string(),
        "-f".to_string(),
        format!("query={}", last_modified_query(owner, names)),
    ];
    let (ok, stdout, stderr) = run_gh_output(&args)?;
    let map = parse_last_modified(&stdout).unwrap_or_default();
    if !ok {
        if map.is_empty() {
            return Err(Error::GitHub(stderr));
        }
        log::warn!(
            "{}: some repositories did not report their last modified time: {}",
            owner,
            stderr.trim()
        );
    }
    Ok(map)
}

fn last_modified_query(owner: &str, names: &[String]) -> String {
    let fields = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            format!(
                "    r{}: repository(owner: {}, name: {}) {{ name updatedAt pushedAt }}",
                i,
                graphql_string(owner),
                graphql_string(name)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("query {{\n{}\n}}\n", fields)
}

/// Quotes a value as a GraphQL string literal, whose syntax is the one of a
/// JSON string.
fn graphql_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn parse_last_modified(json: &str) -> Result<HashMap<String, DateTime<Utc>>> {
    let page: NamedPage = serde_json::from_str(json).map_err(Error::Parse)?;
    Ok(page
        .data
        .into_values()
        .flatten()
        .filter_map(|n| last_modified_of(n.updated_at, n.pushed_at).map(|t| (n.name, t)))
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
pub struct ReleasesNode {
    pub nodes: Vec<ReleaseNode>,
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
    #[serde(rename = "updatedAt", default)]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(rename = "pushedAt", default)]
    pub pushed_at: Option<DateTime<Utc>>,
    #[serde(rename = "isArchived", default)]
    pub is_archived: bool,
    #[serde(rename = "licenseInfo", default)]
    pub license_info: Option<LicenseNode>,
    #[serde(default)]
    pub languages: Option<LanguagesNode>,
    /// The most recent releases of the repository, newest first. The query
    /// asks for a window of them instead of `latestRelease` only, so that
    /// `releases` of the output is rebuilt from scratch on every fetch.
    #[serde(default)]
    pub releases: Option<ReleasesNode>,
    #[serde(rename = "repositoryTopics", default)]
    pub topics: Option<TopicsNode>,
}

impl RepoDetail {
    /// The time the repository was last modified (see [`last_modified_of`]),
    /// comparable with the value reported by [`list_repositories`].
    pub fn last_modified(&self) -> Option<DateTime<Utc>> {
        last_modified_of(self.updated_at, self.pushed_at)
    }
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

    fn at(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn last_modified_takes_the_later_timestamp() {
        let updated = at("2026-10-12T00:13:20Z");
        let pushed = at("2026-10-12T00:13:25Z");
        // totebag: the push is the later one.
        assert_eq!(last_modified_of(Some(updated), Some(pushed)), Some(pushed));
        // 9rules: `updatedAt` lags behind `pushedAt`, hence the push wins.
        assert_eq!(
            last_modified_of(Some(at("2025-08-27T12:10:06Z")), Some(at("2026-07-20T06:41:06Z"))),
            Some(at("2026-07-20T06:41:06Z"))
        );
        // fauxrest: a change that is not a push makes `updatedAt` the later one.
        assert_eq!(
            last_modified_of(Some(at("2026-10-10T00:26:00Z")), Some(at("2026-10-10T00:15:55Z"))),
            Some(at("2026-10-10T00:26:00Z"))
        );
        // an absent timestamp never hides the other one.
        assert_eq!(last_modified_of(Some(updated), None), Some(updated));
        assert_eq!(last_modified_of(None, Some(pushed)), Some(pushed));
        assert_eq!(last_modified_of(None, None), None);
    }

    #[test]
    fn last_modified_query_aliases_every_name() {
        let names = vec!["totebag".to_string(), "jj-cheat-sheet".to_string()];
        let query = last_modified_query("tamada", &names);
        assert_eq!(
            query,
            concat!(
                "query {\n",
                "    r0: repository(owner: \"tamada\", name: \"totebag\") { name updatedAt pushedAt }\n",
                "    r1: repository(owner: \"tamada\", name: \"jj-cheat-sheet\") { name updatedAt pushedAt }\n",
                "}\n"
            )
        );
    }

    #[test]
    fn parse_the_aliased_timestamps() -> Result<()> {
        let json = r#"{"data":{
            "r0":{"name":"totebag","updatedAt":"2026-10-12T00:13:20Z","pushedAt":"2026-10-12T00:13:25Z"},
            "r1":{"name":"9rules","updatedAt":"2025-08-27T12:10:06Z","pushedAt":"2026-07-20T06:41:06Z"},
            "r2":null}}"#;
        let map = parse_last_modified(json)?;
        // the aliases are only a transport: the map is keyed by the name.
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("totebag").copied(), Some(at("2026-10-12T00:13:25Z")));
        assert_eq!(map.get("9rules").copied(), Some(at("2026-07-20T06:41:06Z")));
        // a repository that could not be read is simply absent.
        assert!(!map.contains_key("r2"));
        Ok(())
    }

    #[test]
    fn parse_list_pages() -> Result<()> {
        let json = r#"[{"data":{"user":{"repositories":{"totalCount":3,"nodes":[
            {"name":"fauxrest","description":null,"isArchived":false,"isPrivate":false,
             "createdAt":"2025-05-01T00:00:00Z","updatedAt":"2026-06-30T12:43:00Z",
             "pushedAt":"2026-06-30T12:00:00Z"},
            {"name":"9rules","description":null,"isArchived":false,"isPrivate":false,
             "createdAt":"2024-01-01T00:00:00Z","updatedAt":"2025-08-27T12:10:06Z",
             "pushedAt":"2026-07-20T06:41:06Z"},
            {"name":"totebag","description":"archiver","isArchived":false,"isPrivate":false,
             "createdAt":"2024-01-01T00:00:00Z","updatedAt":"2026-01-15T08:00:00Z",
             "pushedAt":null}
        ],"pageInfo":{"endCursor":"abc","hasNextPage":false}}}}}]"#;
        let map = parse_list(json)?;
        assert_eq!(map.len(), 3);
        // the listing reports the later of `updatedAt` and `pushedAt`.
        assert_eq!(map.get("fauxrest").copied(), Some(at("2026-06-30T12:43:00Z")));
        assert_eq!(map.get("9rules").copied(), Some(at("2026-07-20T06:41:06Z")));
        assert_eq!(map.get("totebag").copied(), Some(at("2026-01-15T08:00:00Z")));
        Ok(())
    }

    const DETAIL_JSON: &str = r#"{"data":{"repository":{
        "name":"totebag","owner":{"login":"tamada"},
        "description":"A tool for archiving/extracting files",
        "homepageUrl":"https://tamada.github.io/totebag",
        "url":"https://github.com/tamada/totebag",
        "createdAt":"2024-01-01T00:00:00Z",
        "updatedAt":"2026-06-01T00:00:00Z",
        "pushedAt":"2026-06-02T00:00:00Z",
        "isArchived":false,"isPrivate":false,"visibility":"PUBLIC","diskUsage":1234,
        "licenseInfo":{"spdx":"mit","name":"MIT License","url":"https://api.github.com/licenses/mit"},
        "primaryLanguage":{"name":"Rust","color":"\\#dea584"},
        "languages":{"nodes":[{"name":"Rust","color":"\\#dea584"},{"name":"Shell","color":null}]},
        "releases":{"nodes":[
          {"name":"Release v0.7.6","tagName":"v0.7.6",
           "url":"https://github.com/tamada/totebag/releases/tag/v0.7.6",
           "description":"bug fix","isDraft":false,
           "createdAt":"2026-05-30T00:00:00Z","publishedAt":"2026-06-01T00:00:00Z"},
          {"name":"Release v0.7.5","tagName":"v0.7.5",
           "url":"https://github.com/tamada/totebag/releases/tag/v0.7.5",
           "description":"new feature","isDraft":false,
           "createdAt":"2026-04-30T00:00:00Z","publishedAt":"2026-05-01T00:00:00Z"}]},
        "repositoryTopics":{"nodes":[{"topic":{"name":"archive"},"url":"https://github.com/topics/archive"}]},
        "collaborators":{"totalCount":2},
        "stargazerCount":10,"forkCount":2,"watchers":{"totalCount":3}}}}"#;

    #[test]
    fn parse_detail_response() -> Result<()> {
        let detail = parse_detail(DETAIL_JSON)?;
        assert_eq!(detail.name, "totebag");
        assert_eq!(detail.owner.login, "tamada");
        assert_eq!(detail.license_info.as_ref().map(|l| l.spdx.as_deref()), Some(Some("mit")));
        assert_eq!(detail.languages.as_ref().map(|l| l.nodes.len()), Some(2));
        let tags = detail
            .releases
            .as_ref()
            .map(|r| r.nodes.iter().map(|n| n.tag_name.as_str()).collect::<Vec<_>>());
        assert_eq!(tags, Some(vec!["v0.7.6", "v0.7.5"]));
        Ok(())
    }

    #[test]
    fn detail_reports_the_same_last_modified_as_the_listing() -> Result<()> {
        let detail = parse_detail(DETAIL_JSON)?;
        // `pushedAt` is the later one here, hence it is the last modified time.
        assert_eq!(detail.last_modified(), Some(at("2026-06-02T00:00:00Z")));
        Ok(())
    }
}
