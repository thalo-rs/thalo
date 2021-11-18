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

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    CREATE TABLE IF NOT EXISTS "event" (
        "id"                BIGSERIAL    PRIMARY KEY,
        "created_at"        TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
        "aggregate_type"    TEXT         NOT NULL,
        "aggregate_id"      TEXT         NOT NULL,
        "sequence"          BIGINT       NOT NULL,
        "event_type"        TEXT         NOT NULL,
        "event_data"        JSONB        NOT NULL,
        UNIQUE ("aggregate_type", "aggregate_id", "sequence")
    );
    CREATE TABLE IF NOT EXISTS "outbox" (
        "id" BIGINT PRIMARY KEY REFERENCES "event"
    );
    CREATE TABLE "bank_account" (
        "account_number" TEXT PRIMARY KEY,
        "last_event_id" BIGINT NOT NULL,
        "last_event_sequence" BIGINT NOT NULL DEFAULT 0,
        "balance" DOUBLE PRECISION NOT NULL DEFAULT 0
    );
EOSQL

echo "############ REPLICATION ##############"                      >> /var/lib/postgresql/data/pg_hba.conf
echo "local   postgres     postgres                          trust" >> /var/lib/postgresql/data/pg_hba.conf
echo "host    postgres     postgres  127.0.0.1/32            trust" >> /var/lib/postgresql/data/pg_hba.conf
echo "host    postgres     postgres  ::1/128                 trust" >> /var/lib/postgresql/data/pg_hba.conf
