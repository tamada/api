pkl2json:
    #! /usr/bin/env bash
    for i in assets/*.pkl ; do
        pkl eval -f json -o data/$(basename $i .pkl).json $i
    done


build:
    cargo build --release

base-product-to-product-json: pkl2json build
    cargo run -- --pretty --base-project-json data/base-products.json > data/products.json

api: base-product-to-product-json
    docker run -v $PWD:/opt quay.io/tama5/fauxrest:latest --dest docs data
