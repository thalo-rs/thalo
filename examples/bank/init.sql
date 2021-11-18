CREATE TABLE "event" (
    "id" BIGSERIAL PRIMARY KEY,
    "aggregate_type" TEXT NOT NULL,
    "aggregate_id" TEXT NOT NULL,
    "sequence" BIGINT NOT NULL,
    "event_type" TEXT NOT NULL,
    "event_data" JSONB NOT NULL,
    UNIQUE ("aggregate_type", "aggregate_id", "sequence")
);

CREATE TABLE "bank_account" (
    "account_number" TEXT PRIMARY KEY,
    "last_event_id" BIGINT NOT NULL,
    "last_event_sequence" BIGINT NOT NULL DEFAULT 0,
    "balance" DOUBLE PRECISION NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS "event" (
    "id" BIGSERIAL PRIMARY KEY,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "aggregate_type" TEXT NOT NULL,
    "aggregate_id" TEXT NOT NULL,
    "sequence" BIGINT NOT NULL,
    "event_type" TEXT NOT NULL,
    "event_data" JSONB NOT NULL,
    UNIQUE ("aggregate_type", "aggregate_id", "sequence")
);

CREATE TABLE IF NOT EXISTS "outbox" ("id" BIGINT PRIMARY KEY REFERENCES "event") COMMENT ON TABLE "event" IS 'Events';

COMMENT ON COLUMN "event"."id" IS 'Auto-incrementing event id';

COMMENT ON COLUMN "event"."aggregate_type" IS 'Aggregate type identifier';

COMMENT ON COLUMN "event"."aggregate_id" IS 'Aggregate instance identifier';

COMMENT ON COLUMN "event"."sequence" IS 'Incrementing number unique where each aggregate instance starts from 0';

COMMENT ON COLUMN "event"."event_type" IS 'Event type identifier, usually SCREAMING_SNAKE_CASE';

COMMENT ON COLUMN "event"."event_data" IS 'Event json payload';