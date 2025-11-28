# Sample Payloads for Integration Demos

These JSON fixtures show representative envelopes for the demo flows. They are mock/local friendly and align with the flow routing (event ingress, bridge, messaging send, optional rebuild event).

- Event: `samples/payloads/build_status_event.json`
  - Build status event with repo, status, commit, duration, correlation, and metadata/idempotency.
- Channel message: `samples/payloads/channel_message.json`
  - Channel message produced by the bridge with a correlation back to the source event.
- Rebuild request event: `samples/payloads/rebuild_request_event.json`
  - Optional event emitted by the Repo Assistant when a rebuild is requested.

Use these as seed payloads when testing bridge/worker logic locally, or as templates when swapping to real providers.
