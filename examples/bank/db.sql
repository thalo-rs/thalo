CREATE TABLE events (
  id SERIAL PRIMARY KEY,
  aggregate_type TEXT NOT NULL,
  aggregate_id TEXT NOT NULL,
  SEQUENCE BIGINT NOT NULL,
  event_type TEXT NOT NULL,
  event_data JSONB NOT NULL,
  UNIQUE (aggregate_type, aggregate_id, SEQUENCE)
);