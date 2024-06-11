#!/bin/bash

sleep 3

DB_NAME="test"
PSQL_USER="postgres"
PSQL_PASSWORD="postgres"
PSQL_HOST="db"
PSQL_PORT="5432"

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
python manage.py runserver 0.0.0.0:8000
