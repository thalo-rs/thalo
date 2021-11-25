FROM acidic9/thalo-db:latest

COPY ./init.sql /docker-entrypoint-initdb.d/
