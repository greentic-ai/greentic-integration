# Chat-Driven Repo Assistant (Messaging ± Event Emission)

What this shows
- Ingest a channel message into a `WorkerRequest` for the Repo Assistant.
- Return a `WorkerResponse` back to the same session/channel.
- Optional branch: emit `greentic.repo.build.request` events when the user requests a rebuild.

Flow file
- `flows/chat_driven/repo_assistant.ygtc`
  - Nodes: `ingress_message` → `to_worker` → `respond` → `done`
  - Optional branch: `to_worker` → `emit_rebuild_event` → `done`
  - Providers/components: `local-messaging`, `demo.worker.repo_assistant`, optional `local-events`

Pack fixture and sample output
- Pack: `packs/integration-demos/pack.json` (scenario `repo_assistant_chat`)
- Golden transcript (sample output): `packs/integration-demos/golden/repo_assistant_chat.json`

Prereqs
- `greentic-runner` installed and on `PATH`
- This repo checked out locally
- Demo config (defaulted in scripts): `configs/demo_local.yaml`

How to run
- With helper script:
  ```bash
  ./scripts/run_repo_assistant_demo.sh
  ```
- Directly:
  ```bash
  greentic-runner run-flow \
    --flow-file flows/chat_driven/repo_assistant.ygtc \
    --config configs/demo_local.yaml
  ```
- To view the sample ChannelMessageEnvelope used for ingress, set `SHOW_PAYLOAD=1 ./scripts/run_repo_assistant_demo.sh` (prints `samples/payloads/channel_message.json`). The optional rebuild event payload lives at `samples/payloads/rebuild_request_event.json`.
- To replay the sample message via the runner proxy (local stub or HTTP server), use:
  ```bash
  ./scripts/replay_repo_assistant_payload.sh                # local stub
  USE_SERVER=1 ./scripts/replay_repo_assistant_payload.sh   # POST to server (default http://localhost:8080)
  ```

Toggling the optional rebuild branch
- The flow routes `rebuild_requested` from the worker to `emit_rebuild_event`.
- To skip event emission entirely, remove or ignore that route in your config/run; the core messaging path still works.
- To exercise it, ensure `local-events` (or your real events provider) is available and emits to `greentic.repo.build.request`.

Sample payloads
- ChannelMessageEnvelope example (ingress): `samples/payloads/channel_message.json`
- Rebuild request EventEnvelope example (optional branch): `samples/payloads/rebuild_request_event.json`
