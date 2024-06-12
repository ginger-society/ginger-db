version: '3'

services:
    {{name}}-runtime:
        image: db-compose-runtime:latest
        ports:
            - 8000:8000
        environment:
            - DB_NAME={{name}}-db
            - DB_USERNAME=postgres
            - DB_PASSWORD=postgres
            - DB_HOST={{name}}-db
            - DB_PORT={{port}}
        volumes:
            - ./models.py:/app/src/models.py
            - ./admin.py:/app/src/admin.py
        depends_on:
            - {{name}}-db
    {{name}}-db:
        image: postgres:14.1-alpine
        restart: always
        environment:
            - POSTGRES_USER=postgres
            - POSTGRES_PASSWORD=postgres
        ports:
            - {{port}}:5432
        volumes:
            - ./pgsql:/var/lib/postgresql/data
