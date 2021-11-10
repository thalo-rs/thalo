CREATE TABLE events (
  id BIGSERIAL PRIMARY KEY,
  aggregate_type TEXT NOT NULL,
  aggregate_id TEXT NOT NULL,
  sequence BIGINT NOT NULL,
  event_type TEXT NOT NULL,
  event_data JSONB NOT NULL,
  UNIQUE (aggregate_type, aggregate_id, sequence)
);