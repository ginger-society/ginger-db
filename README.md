This is a utility CLI app which can be used for

1. Automated DB migrations in pipelines
2. Setup local DB for running copy of DB with the predefined schema during development


openapi-generator generate -g rust \
 -i http://localhost:8000/swagger/\?format\=openapi \
 -o schema-client \
 --additional-properties=useSingleRequestParameter=true,packageName=schemaClient