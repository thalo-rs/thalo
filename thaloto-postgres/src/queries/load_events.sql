SELECT
  "id",
  "created_at",
  "aggregate_type",
  "aggregate_id",
  "sequence",
  "event_type",
  "event_data"
FROM "event"
WHERE
  "aggregate_type" = $1 AND
  (CAST($2 as TEXT) IS NULL OR "aggregate_id" = $2)
ORDER BY "sequence" ASC;