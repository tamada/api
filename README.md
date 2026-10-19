# API

This repository provides psuede REST API.

[![Powered by: FauxREST](https://tamada.github.io/fauxrest/powered-by.svg)](https://github.com/tamada/fauxrest)

Operational notes for the maintainer, in Japanese (how a notification from a product
repository is handled, and how to register a new product):
[.github/docs/ja/README.md](.github/docs/ja/README.md).

## Product Merger

The `product-merger` is a CLI tool that reads the base products information from JSON file, and 
obtains their additional information from GitHub etc.
Then, merge these information into a single JSON file.

The `product-merger` uses [GitHub API](https://docs.github.com/en/rest) to obtain additional information via [`gh`](https://cli.github.com/) command.
Therefore, we should prepare `gh` command before using it.

### Usage

```sh
product-merger [OPTIONs] <JSON> [<OUTPUT>]
OPTIONs
    -b, --base-project-json    if specified, all the entries in the given JSON
            file are parsed as the base product information. if not, each
            entry is recognized automatically: entries having the product
            specific keys (url, repository, sbom, releases, ...) are parsed
            as the full product information, and the others as the base
            product information (mixed JSON is acceptable).
        --cache <JSON|URL>     reuse the values of the previous build from the
            given JSON file, or from the given URL (http:// or https://, read
            with `curl`), and fetch only the repositories modified since.
            The published API itself is such a JSON:
            https://tamada.github.io/api/products/index.json. A cache that
            cannot be read or parsed is reported as a warning and every
            product is fetched.
        --no-cache             ignore --cache and fetch every product from
            GitHub.
    -l, --level <LEVEL>        set the log level to <LEVEL> (default: info).
    -p, --pretty               print the output in pretty format.
    -h, --help                 print this message.
JSON
    the input JSON file path. if the '-' is specified, the input is read from stdin.
OUTPUT
    the output JSON file path. if not specified, or specified as '-', 
    the output is printed to stdout.
```

### How it works

1. Parses the input JSON as an array of products and/or base products
   (see `assets/base-products.pkl` for the definitions). This array alone
   decides which products the output holds; `--cache` never adds one.
2. Reads the cache given by `--cache`, if any: the output of a previous build,
   from which only the values that come from GitHub are reused, and only for
   the repositories that have not been modified since.
3. Obtains the last modified time of the repositories of each owner in one
   `gh api graphql` request, which names the configured repositories (one
   aliased `repository` field each) instead of listing the repositories of
   the owner. A `user(login:)` listing would reach well beyond the products:
   its default affiliations cover every repository the user merely
   collaborates on, so it also names repositories owned by other people and
   by organizations, which is enough for an organization policy the token
   does not satisfy to refuse the whole request. Naming the repositories
   needs no access beyond the one the detail query below already has.

   The last modified time is the **later of `updatedAt` and `pushedAt`**.
   Neither is above the other: `updatedAt` also follows the changes that are
   not a push (the description, the topics), while `pushedAt` is the only one
   that follows a push to a non-default branch, and a repository may well
   report an `updatedAt` older than its `pushedAt`. Both this value and the
   `last_updated` written to the output are the same reduction of the two, so
   that the two sides of the comparison below are comparable; comparing two
   different fields would skip a repository forever whenever the stored one
   happened to run ahead.
4. For each entry, fetches the repository detail by
   `assets/queries/products.graphql` and builds the product information,
   unless the entry has a cached (or, for a full product in the input, its
   own) `last_updated` that is not older than the time of step 3. In that
   case the values that come from GitHub (the description, the releases, the
   license, the languages, the topics) are reused as they are, while `links`,
   `logo` and `overrides` are applied again from the input, so that a change
   of the configuration shows up without a fetch. A bare `www` or `repository`
   keeps the URL GitHub reported by reading it back from the cached links.
5. Overwrites the fetched values with the entries of `overrides`, key by key
   and as a whole value rather than merged (e.g.,
   `{"overrides": {"description": "..."}}` replaces the fetched description).
   The `owner` and `name` keys are not overridable since they identify the
   repository, and neither is `overrides` itself. Overriding one of the retired
   keys (`logo`, `url`, `repository`, `sbom`) has no effect either, since they
   are dropped from the output in favor of `links`; replace `links` instead.
   The `overrides` map itself is kept in the output so that it is re-applied on
   every refresh.
6. Resolves `links` into a single array of `{"label": ..., "url": ...}`,
   which holds every URL of the product. An entry given as an object is kept
   as it is, and an entry given as a bare link type is filled in as follows:

   | link type | URL |
   |-----------|-----|
   | `www` | `homepageUrl` of the repository, or `https://<owner>.github.io/<name>` when it is empty |
   | `repository` | `url` of the repository, or `https://github.com/<owner>/<name>` when GitHub is not accessed |
   | `sbom` | `https://api.github.com/repos/<owner>/<name>/dependency-graph/sbom` (GitHub does not report it) |

   The other types (`logo`, `docs`, `registry`, `container`, `artifact`) have
   nothing to fill in, so they must be written as an object with their own
   `url`; a bare one is logged and dropped. Writing a type as an object also
   pins its URL: `{"label": "www", "url": ...}` is kept as it is instead of
   taking `homepageUrl`. The `logo` key of a base product is a shorthand, and
   becomes the first `logo` link of the output.

   A link may also carry `ref`, the pull reference of an OCI artifact or a
   container image, e.g.
   `{"label": "artifact", "ref": "quay.io/tama5/pelicode-db:v1"}`. It is what
   `oras pull` or `docker pull` takes, as opposed to `url`, which is the page
   to open in a browser. The things that are only pulled have no page of their
   own, hence `url` is optional on a link that has a `ref`; a link carrying
   neither is logged and dropped. Both keys are written only when present, so
   the links that use neither are unchanged.
7. Prints the resultant array of the full products in JSON. Every URL lives
   in `links`: the output has no `logo`, `url`, `repository`, nor `sbom` key
   of its own. It contains `last_updated`, therefore the next run given this
   output as its `--cache` fetches only the products whose repositories were
   modified since.

`releases` holds the ten most recent releases of the repository, drafts
excluded, newest first; a release without a name of its own is named after its
tag. The whole array is rebuilt from the fetch every time, so it is a value
that any refetch reproduces rather than a history that only the previous
output holds: a missed build cannot lose a release, and `--no-cache` rebuilds
every one of them.

The failures on individual products (e.g., renamed or removed repositories)
are logged and do not abort the whole process: the cached information is kept
when a cache holds the product, and the defaults derived from `owner` and
`name` otherwise. A fetch that reports no timestamp leaves `last_updated`
absent on purpose, so that the next run fetches the product again instead of
keeping a partial value forever.
