CREATE KEYSPACE IF NOT EXISTS thalo
WITH replication = {
	'class': 'SimpleStrategy',
	'replication_factor': 1
}
