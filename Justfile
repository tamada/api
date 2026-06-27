echo message:
    @echo {{ message }}

pkl2json: pkl2json_tamada pkl2json_tamadalab

pkl2json_tamada: (_pkl2json_impl "tamada")

pkl2json_tamadalab: (_pkl2json_impl "tamadalab")

_pkl2json_impl base:
    #! /usr/bin/env sh -x
    echo "pkl2json {{base}}"
    mkdir -p {{base}}/data
    for i in {{base}}/assets/*[^_model].pkl
    do
        name=$(basename $i)
        pkl eval -f json $i -o {{base}}/data/${name%.*}.json
    done

generate: generate_tamada generate_tamadalab

generate_tamada output_dir="docs": pkl2json_tamada
    @echo "generate Haruaki Tamada's REST API files to {{output_dir}}"
    export PYTHONPATH=$PYTHONPATH:$(pwd)/scripts; \
    python3 ./scripts/translate_tamada.py --source-dir tamada/data --output-dir {{output_dir}}

generate_tamadalab output_dir="tamadalab/api": pkl2json_tamadalab
    @echo "generate TamadaLab's REST API files to {{output_dir}}"
    export PYTHONPATH=$PYTHONPATH:$(pwd)/scripts; \
    python3 ./scripts/translate_tamadalab.py --source-dir tamadalab/data --output-dir {{output_dir}}

test_tamada output_dir="docs": generate_tamada
    @echo "run verification tests for tamada"
    export PYTHONPATH=$PYTHONPATH:$(pwd)/scripts; \
    python3 ./scripts/test_deploy.py --tamada-api {{output_dir}}

test_tamadalab output_dir="tamadalab/api": generate_tamadalab
    @echo "run verification tests for tamadalab"
    export PYTHONPATH=$PYTHONPATH:$(pwd)/scripts; \
    python3 ./scripts/test_deploy.py --tamadalab-api {{output_dir}}

test: test_tamada test_tamadalab
