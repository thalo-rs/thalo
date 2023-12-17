CREATE TABLE IF NOT EXISTS thalo.event_sequence_counter (
	stream_name text PRIMARY KEY,
	last_sequence bigint
)
