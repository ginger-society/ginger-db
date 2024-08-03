This is a utility CLI app which can be used for

1. Automated DB migrations in pipelines
2. Setup local DB for running copy of DB with the predefined schema during development

run `ginger-connector connect dev` to generate schema service client



Notes:


build image using :

This is the image used to migrate and also run the admin view server

```sh
cd runner-image
docker build . -t gingersociety/db-compose-runtime # build
docker push gingersociety/db-compose-runtime:latest # push
```

And to build the migrator image

```sh
cd runner-image
docker build . -t gingersociety/db-compose-migrator --file Dockerfile.migrator # build
docker push gingersociety/db-compose-migrator:latest # push
```

In the pipeline of the db repo that it creates , it should execute the following commands 

```sh
docker run -e DB_NAME=NAME -e DB_USERNAME=USERNAME -e DB_PASSWORD=PASS -e DB_HOST=HOST -e DB_PORT=PORT -v $(pwd)/models.py:/app/src/models.py -v $(pwd)/admin.py:/app/src/admin.py -v $(pwd)/migrations:/app/src/migrations gingersociety/db-compose-migrator:latest

```