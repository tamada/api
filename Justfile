pkl2json: pkl2json_tamada pkl2json_tamadalab

pkl2json_tamada: (_pkl2json_impl "tamada")

pkl2json_tamadalab: (_pkl2json_impl "tamadalab")

_pkl2json_impl target:
    #! /usr/bin/env sh
    cd {{target}}
    mkdir -p data
    for i in assets/*[^_model].pkl
    do
        name=$(basename $i)
        pkl eval -f json $i -o data/${name%.*}.json
    done
