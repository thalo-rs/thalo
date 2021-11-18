# Outbox Relay

A cli tool for relaying events from an outbox table to a kafka/redpanda event broker.

### Example

```bash
outbox-relay -d postgres://bank:8Wu87y5jlq7A@127.0.0.1:5435/bank -r 127.0.0.1:9092
```
