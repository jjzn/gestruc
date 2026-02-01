#!/bin/sh

echo 'Creating DB with schema'
sqlite3 $1 < "$(dirname $0)/schema.sql"

echo 'Populating DB with mock data'
sqlite3 $1 < "$(dirname $0)/populate_mock.sql"
