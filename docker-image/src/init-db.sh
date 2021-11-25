#!/bin/bash
set -e

echo "############ REPLICATION ##############"      >> /var/lib/postgresql/data/postgresql.conf
echo "# MODULES"                                    >> /var/lib/postgresql/data/postgresql.conf
echo "shared_preload_libraries = 'wal2json'"        >> /var/lib/postgresql/data/postgresql.conf
echo ""                                             >> /var/lib/postgresql/data/postgresql.conf
echo "# REPLICATION"                                >> /var/lib/postgresql/data/postgresql.conf
echo "wal_level = logical"                          >> /var/lib/postgresql/data/postgresql.conf
echo "max_wal_senders = 4"                          >> /var/lib/postgresql/data/postgresql.conf
echo "max_replication_slots = 4"                    >> /var/lib/postgresql/data/postgresql.conf

echo "############ REPLICATION ##############"                      >> /var/lib/postgresql/data/pg_hba.conf
echo "local   postgres     postgres                          trust" >> /var/lib/postgresql/data/pg_hba.conf
echo "host    postgres     postgres  127.0.0.1/32            trust" >> /var/lib/postgresql/data/pg_hba.conf
echo "host    postgres     postgres  ::1/128                 trust" >> /var/lib/postgresql/data/pg_hba.conf
