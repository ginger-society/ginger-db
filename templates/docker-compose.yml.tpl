version: '3'

services:
    {{name}}-runtime:
        image: gingersociety/db-compose-runtime:latest
        ports:
            - {{studio_port}}:8000
        environment:
            - DB_NAME={{name}}-db
            - DB_USERNAME={{db_username}}
            - DB_PASSWORD={{db_password}}
            - DB_HOST={{name}}-db
            - DB_PORT=5432
        volumes:
            - ./models.py:/app/src/models.py
            - ./admin.py:/app/src/admin.py
        depends_on:
            - {{name}}-db
    {{name}}-db:
        image: postgres:14.1-alpine
        restart: always
        environment:
            - POSTGRES_USER={{db_password}}
            - POSTGRES_PASSWORD={{db_password}}
        ports:
            - {{port}}:5432
        volumes:
            - ./pgsql:/var/lib/postgresql/data
