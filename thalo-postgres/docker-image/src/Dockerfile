FROM postgres:14

RUN export PATH=/usr/lib/postgresql/14/bin:$PATH
RUN apt-get update
RUN apt-get -y install build-essential
RUN apt-get -y install git
RUN apt-get -y install postgresql-server-dev-14

WORKDIR "/home/postgres"
RUN git clone https://github.com/eulerto/wal2json -b master --single-branch

WORKDIR "/home/postgres/wal2json"
RUN make
RUN make install
RUN rm -rf /home/postgre/swal2json

COPY ./init-db.sh /docker-entrypoint-initdb.d/
COPY ./init-es.sql /docker-entrypoint-initdb.d/
