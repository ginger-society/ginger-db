version: '3'

services:
    runtime:
        image: db-compose-runtime:latest
        ports:
            - 8000:8000
        environment:
            - env=prod
            - DB_NAME=django_2
            - DB_USERNAME=postgres
            - DB_PASSWORD=postgres
            - DB_HOST=db
            - ALLOWED_HOSTS=localhost
            - CSRF_TRUSTED_ORIGINS=http://localhost
        volumes:
            - ./models.py:/app/src/models.py
            - ./admin.py:/app/src/admin.py
        depends_on:
            - db
    db:
        image: postgres:14.1-alpine
        restart: always
        environment:
            - POSTGRES_USER=postgres
            - POSTGRES_PASSWORD=postgres
        ports:
            - 5432:5432
        volumes:
            - ./pgsql:/var/lib/postgresql/data
