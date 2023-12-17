CREATE KEYSPACE IF NOT EXISTS thalo
WITH replication = {
	'class': 'SimpleStrategy',
	'replication_factor': 1
};

CREATE TABLE IF NOT EXISTS thalo.event_store (
    stream_name text,
    sequence bigint,
    id uuid,
    event_type text,
    data blob,
    timestamp timestamp,
    PRIMARY KEY ((stream_name), sequence)
) WITH CLUSTERING ORDER BY (sequence ASC);

CREATE TABLE IF NOT EXISTS thalo.event_sequence_counter (
	stream_name text PRIMARY KEY,
	last_sequence bigint
);
