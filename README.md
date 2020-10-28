# Actix-sse-pubsub

This is derived from the sse in https://github.com/actix/examples which is under the Apache license.

## Usage

```
cargo run --release
```

Events are at ``/events/{channel}``

To broadcast, send a post request to ``/broadcast`` with json content of:
```json
{
  "channel": "channel name",
  "msg": "message to broadcast"
}
```

To benchmark, run multiple instances of

```
python3 benchmark.py
```

