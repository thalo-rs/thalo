INSERT INTO thalo.events (
    stream_name,
    sequence,
    global_sequence,
    id,
    event_type,
    data,
    timestamp,
    bucket
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
IF NOT EXISTS
