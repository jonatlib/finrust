#!/bin/bash

CWD=$(pwd)
ORIGINAL="/Users/libor/Projects/Home/money_project/src/money_project"

DUMP_TARGET="${CWD}/dumped.json"
DATABASE_FILE="${CWD}/finrust.sqlite"
DATABASE="sqlite://${DATABASE_FILE}"

rm "${DUMP_TARGET}"
rm "${DATABASE_FILE}"

cd "${ORIGINAL}"
uv run python ./manage.py dumpdata --format=json -o "${DUMP_TARGET}"

cd "${CWD}"
touch "${DATABASE_FILE}"
cargo run -- init-db -d "${DATABASE}"
cargo run -- import-djang --json-path "${DUMP_TARGET}" --database-url "${DATABASE}"
