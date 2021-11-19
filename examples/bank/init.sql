CREATE TABLE "bank_account" (
    "account_number"      TEXT PRIMARY KEY,
    "last_event_id"       BIGINT NOT NULL,
    "last_event_sequence" BIGINT NOT NULL DEFAULT 0,
    "balance"             DOUBLE PRECISION NOT NULL DEFAULT 0
);