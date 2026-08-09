# API

This repository provides psuede REST API.

[![Powered by](https://img.shields.io/badge/Powered_by-FauxREST-brightgreen)](https://github.com/tamada/fauxrest).

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
   (see `assets/base-products.pkl` for the definitions).
2. Runs `gh api graphql` with `assets/queries/product_list.graphql`
   (paginated) once per owner, to obtain the last updated time (`updatedAt`)
   of all the repositories of the owner.
3. For each entry, if it is a base product, or its `last_updated` is older
   than the repository's `updatedAt`, fetches the repository detail by
   `assets/queries/products.graphql` and builds/updates the product
   information. Otherwise, the entry is kept as-is.
4. Overwrites the fetched values with the entries of `overrides` (e.g.,
   `{"overrides": {"url": "https://crates.io/crates/pick-a-boo"}}` replaces
   the fetched `url`). The `owner` and `name` keys are not overridable since
   they identify the repository. The `overrides` map itself is kept in the
   output so that it is re-applied on every refresh.
5. Resolves `links` into a single array of `{"label": ..., "url": ...}`,
   which holds every URL of the product. An entry given as an object is kept
   as it is, and an entry given as a bare link type is filled in as follows:

   | link type | URL |
   |-----------|-----|
   | `www` | `homepageUrl` of the repository, or `https://<owner>.github.io/<name>` when it is empty |
   | `repository` | `url` of the repository, or `https://github.com/<owner>/<name>` when GitHub is not accessed |
   | `sbom` | `https://api.github.com/repos/<owner>/<name>/dependency-graph/sbom` (GitHub does not report it) |

   The other types (`logo`, `docs`, `registry`, `container`) have nothing to
   fill in, so they must be written as an object with their own `url`; a bare
   one is logged and dropped. Writing a type as an object also pins its URL:
   `{"label": "www", "url": ...}` is kept as it is instead of taking
   `homepageUrl`. The `logo` key of a base product is a shorthand, and becomes
   the first `logo` link of the output.
6. Prints the resultant array of the full products in JSON. Every URL lives
   in `links`: the output has no `logo`, `url`, `repository`, nor `sbom` key
   of its own. It contains `last_updated`, therefore the next run updates
   only the products whose repositories were updated after the previous run.

The failures on individual products (e.g., renamed or removed repositories)
are logged and do not abort the whole process; the previous information is
kept for such products.
