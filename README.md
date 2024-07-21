This is a utility CLI app which can be used for

1. Automated DB migrations in pipelines
2. Setup local DB for running copy of DB with the predefined schema during development

run `ginger-connector connect dev` to generate schema service client

Notes:
build image using : docker build . -t db-compose-runtime
To push : docker push gingersociety/db-compose-runtime:latest