FROM acidic9/awto-es:latest

COPY ./init.sql /docker-entrypoint-initdb.d/
