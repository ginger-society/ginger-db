version: '3'
services:
    {% for db in databases %}
    {% if db.enable %}
    {% if db.db_type == "rdbms" %}
    {% if db.id %}
    {{ db.name }}-runtime:
        image: gingersociety/db-compose-runtime:latest
        command: ["/app/run.sh"]
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
    {% else %}
    {{ db.name }}-pgweb:
        image: sosedoff/pgweb
        restart: always
        ports:
            - {{ db.studio_port }}:8081
        environment:
            - DATABASE_URL=postgres://postgres:postgres@{{ db.name }}-db:5432/{{ db.name }}-db?sslmode=disable
        depends_on:
            - {{ db.name }}-db
    {% endif %}
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
        image: redis:7-alpine
        restart: always
        command: ["redis-server", "--save", "", "--appendonly", "no"]
        healthcheck:
            test: ["CMD", "redis-cli", "ping"]
            interval: 1s
            timeout: 3s
            retries: 50
        ports:
            - {{ db.port }}:6379
    {% elif db.db_type == "messagequeue" %}
    {{ db.name }}-messagequeue:
        image: rabbitmq:3-management
        ports:
            - {{ db.port }}:5672
            - {{ db.studio_port }}:15672
        environment:
            RABBITMQ_DEFAULT_USER: user
            RABBITMQ_DEFAULT_PASS: password
    {% endif %}
    {% endif %}
    {% endfor %}