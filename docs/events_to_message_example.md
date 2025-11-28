# Build Status Notifications (Events → Message)

What this shows
- Listens for `greentic.repo.build.status` events from a local/mock events provider.
- Bridges each event into a channel-friendly `ChannelMessageEnvelope`.
- Sends the message via a mock/local messaging provider (no Teams/Slack hardwiring).

Flow file
- `flows/events_to_message/build_status_notifications.ygtc`
  - Nodes: `event_ingress` → `bridge_to_message` → `send_message` → `done`
  - Providers/components: `local-events`, `demo.bridge.events_to_message`, `local-messaging`

Pack fixture and sample output
- Pack: `packs/integration-demos/pack.json` (scenario `build_status_notification`)
- Golden transcript (sample output): `packs/integration-demos/golden/build_status_notification.json`

Prereqs
- `greentic-runner` installed and on `PATH`
- This repo checked out locally
- Demo config (defaulted in scripts): `configs/demo_local.yaml`

How to run
- With helper script:
  ```bash
  ./scripts/run_build_status_demo.sh
  ```
- Directly:
  ```bash
  greentic-runner run-flow \
    --flow-file flows/events_to_message/build_status_notifications.ygtc \
    --config configs/demo_local.yaml
  ```
- To see the sample EventEnvelope that pairs with this flow, set `SHOW_PAYLOAD=1 ./scripts/run_build_status_demo.sh` (payload printed from `samples/payloads/build_status_event.json`).
- To replay the sample payload via the runner proxy (local stub or HTTP server), use:
  ```bash
  ./scripts/replay_build_status_payload.sh                # local stub
  USE_SERVER=1 ./scripts/replay_build_status_payload.sh   # POST to server (default http://localhost:8080)
  ```

Using real providers
- Swap the provider/component IDs in `configs/demo_local.yaml` (or your own config) to point at real providers (e.g., `nats-core` for events, `teams-main` for messaging).
- The flow itself stays the same; only the provider bindings change.

Sample payloads
- EventEnvelope example: `samples/payloads/build_status_event.json`
- ChannelMessageEnvelope example (bridge output): `samples/payloads/channel_message.json`
