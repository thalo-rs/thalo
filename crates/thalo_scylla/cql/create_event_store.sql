CREATE TABLE IF NOT EXISTS thalo.event_store (
    stream_name text,
    sequence bigint,
    id uuid,
    event_type text,
    data blob,
    timestamp timestamp,
    PRIMARY KEY ((stream_name), sequence)
) WITH CLUSTERING ORDER BY (sequence ASC)
