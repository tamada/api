pkl2json:
    #! /usr/bin/env bash
    for i in assets/*.pkl ; do
        pkl eval -f json -o data/$(basename $i .pkl).json $i
    done


build:
    cargo build --release

# The published API is read back as the cache, exactly as the workflow does, so that
# only the repositories modified since the last deploy are fetched from GitHub. Add
# --no-cache to fetch every one of them.
cache_url := "https://tamada.github.io/api/products/index.json"

base-product-to-product-json *OPTS: pkl2json build
    cargo run --release -- --pretty --base-project-json --cache {{cache_url}} {{OPTS}} data/base-products.json > data/products.json

api: base-product-to-product-json
    docker run -v $PWD:/opt quay.io/tama5/fauxrest:latest --dest docs data
