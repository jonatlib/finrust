#!/bin/bash

set -xe

CWD=$(pwd)
ORIGINAL="/Users/libor/Projects/Home/money_project/src/money_project"

DUMP_TARGET="${CWD}/dumped.json"
DATABASE_FILE="${CWD}/data/finrust.db"
DATABASE="sqlite://${DATABASE_FILE}"

rm "${DUMP_TARGET}" || true
rm "${DATABASE_FILE}" || true

cd "${ORIGINAL}"
uv run python ./manage.py dumpdata --format=json -o "${DUMP_TARGET}"

cd "${CWD}"
touch "${DATABASE_FILE}"
cargo run -- init-db -d "${DATABASE}"
#cargo run --bin migration up
cargo run -- import-django --json-path "${DUMP_TARGET}" --database-url "${DATABASE}" --overlay ./data/account_overlay.yaml
