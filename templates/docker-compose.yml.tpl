version: '3'

services:
    {% for db in databases %}
    {% if db.db_type == "rdbms" %}
    {{ db.name }}-runtime:
        image: gingersociety/db-compose-runtime:latest
        ports:
            - {{ db.studio_port }}:8000
        environment:
            - DB_NAME={{ db.name }}-db
            - DB_USERNAME=postgres
            - DB_PASSWORD=postgres
            - DB_HOST={{ db.name }}-db
            - DB_PORT=5432
        volumes:
            - ./{{ db.name }}/models.py:/app/src/models.py
            - ./{{ db.name }}/admin.py:/app/src/admin.py
            - ./{{ db.name }}/migrations:/app/src/migrations
        depends_on:
            - {{ db.name }}-db
    {{ db.name }}-db:
        image: postgres:14.1-alpine
        restart: always
        environment:
            - POSTGRES_USER=postgres
            - POSTGRES_PASSWORD=postgres
        ports:
            - {{ db.port }}:5432
        volumes:
            - ./{{ db.name }}/pgsql:/var/lib/postgresql/data
    {% elif db.db_type == "documentdb" %}
    {{ db.name }}-mongodb:
        image: mongo:latest
        environment:
            MONGO_INITDB_ROOT_USERNAME: mongo
            MONGO_INITDB_ROOT_PASSWORD: mongo
        ports:
            - {{ db.port }}:27017
        volumes:
            - ./{{ db.name }}/mongodb:/data/db

    {{ db.name }}-mongo-gui:
        image: ugleiton/mongo-gui
        platform: linux/amd64
        restart: always
        ports:
            - "{{ db.studio_port }}:4321"
        environment:
            - MONGO_URL=mongodb://mongo:mongo@{{ db.name }}-mongodb:27017
    {% elif db.db_type == "cache" %}
    {{ db.name }}-redis:
        image: bitnami/redis:6.2.5
        restart: always
        environment:
            ALLOW_EMPTY_PASSWORD: "yes"
        healthcheck:
            test: redis-cli ping
            interval: 1s
            timeout: 3s
            retries: 50
        ports:
            - {{ db.port }}:6379
    {% endif %}
    {% endfor %}
