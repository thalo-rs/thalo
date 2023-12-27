SELECT
    stream_name,
    sequence,
    id,
    event_type,
    data,
    timestamp
FROM thalo.event_store
WHERE stream_name = ?
  AND sequence >= ?
ORDER BY sequence ASC
