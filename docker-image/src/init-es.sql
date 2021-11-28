/* Event table */
CREATE TABLE IF NOT EXISTS "event" (
  "id"             BIGSERIAL              PRIMARY KEY,
  "created_at"     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "aggregate_type" TEXT                      NOT NULL,
  "aggregate_id"   TEXT                      NOT NULL,
  "sequence"       BIGINT                    NOT NULL,
  "event_type"     TEXT                      NOT NULL,
  "event_data"     JSONB                     NOT NULL,
  UNIQUE ("aggregate_type", "aggregate_id", "sequence")
);

COMMENT ON TABLE  "event"                  IS 'Events';
COMMENT ON COLUMN "event"."id"             IS 'Auto-incrementing event id';
COMMENT ON COLUMN "event"."created_at"     IS 'Event timestamp';
COMMENT ON COLUMN "event"."aggregate_type" IS 'Aggregate type identifier';
COMMENT ON COLUMN "event"."aggregate_id"   IS 'Aggregate instance identifier';
COMMENT ON COLUMN "event"."sequence"       IS 'Incrementing number unique where each aggregate instance starts from 0';
COMMENT ON COLUMN "event"."event_type"     IS 'Event type identifier, usually SCREAMING_SNAKE_CASE';
COMMENT ON COLUMN "event"."event_data"     IS 'Event json payload';

/* Outbox table */
CREATE TABLE IF NOT EXISTS "outbox" (
  "id" BIGINT PRIMARY KEY REFERENCES "event"
);

COMMENT ON TABLE  "outbox"      IS 'Outbox';
COMMENT ON COLUMN "outbox"."id" IS 'Event id';

/* Projection table */
CREATE TABLE IF NOT EXISTS "projection" (
  "id"             TEXT   NOT NULL,
  "type"           TEXT   NOT NULL,
  "event_id"       BIGINT NOT NULL,
  "event_sequence" BIGINT NOT NULL,
  PRIMARY KEY ("id", "type")
);

COMMENT ON TABLE  "projection"                  IS 'Projection';
COMMENT ON COLUMN "projection"."id"             IS 'Projection id';
COMMENT ON COLUMN "projection"."type"           IS 'Projection type';
COMMENT ON COLUMN "projection"."event_id"       IS 'Projection last event id';
COMMENT ON COLUMN "projection"."event_sequence" IS 'Projection last event sequence';
