SELECT
  "id",
  "created_at",
  "aggregate_type",
  "aggregate_id",
  "sequence",
  "event_data"
FROM "event"
WHERE
  "id" = ANY($1::INT[])
ORDER BY "sequence" ASC;