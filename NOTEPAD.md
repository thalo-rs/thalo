# Notepad

A notepad for me to document resources, general event sourcing information, and ideas.
This is not intended to be any real documentation.

### Versioning Notes

> A new version of an event must be convertible from the old version of the event.
> If not, it is not a new version of the event but rather a new event.

### Serial event ID can cause events to be skipped

Sequences in Postgres are not transactional, which means queries made between two commits can result in a missed event.

https://github.com/WegenenVerkeer/akka-persistence-postgresql#write-strategies

https://github.com/akka/akka-persistence-jdbc/issues/96

message-db solves for this by locking categories on write, and not allowing you to query an $all stream (only categories).

### Idempotent Command Handlers

Command handlers must be idempotent. Commands will be recycled typically on startup, and command handlers are expected
to handle this accordingly to avoid duplicate processing.

The typical way of doing this is to store a sequence in the entity state, and compare the command position with this sequence.

The `Context` type has a helper for this called `Context::processed`.
