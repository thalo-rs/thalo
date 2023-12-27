SELECT max(sequence)
FROM thalo.event_store
WHERE stream_name = ?
