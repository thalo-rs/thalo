INSERT INTO thalo.event_store (
    stream_name,
    sequence,
    id,
    event_type,
    data,
    timestamp
) VALUES (?, ?, ?, ?, ?, ?)
IF NOT EXISTS
