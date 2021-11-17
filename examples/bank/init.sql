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