version: '3'

services:
    {%  if create_mongodb %}
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
    {% endif %}
    {%  if create_mongodb %}
    {{name}}-mongodb:
        image: mongo:latest
        environment:
            MONGO_INITDB_ROOT_USERNAME: {{mongo_username}}
            MONGO_INITDB_ROOT_PASSWORD: {{mongo_password}}
        ports:
            - {{mongo_port}}:27017
        volumes:
            - ./mongodb:/data/db
    {% endif %}
    {%  if create_redis %}
    {{name}}-redis:
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
            - {{redis_port}}:6379
    {% endif %}