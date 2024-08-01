#!/bin/bash

sleep 5

DB_NAME=${DB_NAME:-"test"}
PSQL_USER=${DB_USERNAME:-"postgres"}
PSQL_PASSWORD=${DB_PASSWORD:-"postgres"}
PSQL_HOST=${DB_HOST:-"db"}
PSQL_PORT=${DB_PORT:-"5432"}

export PGPASSWORD=$PSQL_PASSWORD

if ! psql -U $PSQL_USER -h $PSQL_HOST -p $PSQL_PORT -lqt | cut -d \| -f 1 | grep -qw $DB_NAME; then
    createdb -U $PSQL_USER -h $PSQL_HOST -p $PSQL_PORT $DB_NAME
    echo "Database $DB_NAME created."
else
    echo "Database $DB_NAME already exists."
fi

unset PGPASSWORD

python manage.py makemigrations
python manage.py migrate
