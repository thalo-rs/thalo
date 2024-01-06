CREATE TABLE IF NOT EXISTS thalo.events (
    stream_name text,
    sequence bigint,
    global_sequence bigint,
    id uuid,
    event_type text,
    data blob,
    timestamp timestamp,
    bucket bigint,
    PRIMARY KEY (stream_name, sequence, global_sequence, bucket)
) WITH CLUSTERING ORDER BY (sequence ASC, global_sequence ASC);
  AND cdc = { 'enabled': true };


CREATE MATERIALIZED VIEW events_by_global_sequence AS
SELECT * FROM thalo.events
WHERE stream_name IS NOT NULL AND sequence IS NOT NULL AND global_sequence IS NOT NULL AND bucket IS NOT NULL
PRIMARY KEY (bucket, global_sequence, sequence, stream_name)
WITH CLUSTERING ORDER BY (global_sequence ASC, sequence ASC, stream_name ASC);
