# Integration Demo Pack

This pack bundles the mock-first integration flows demonstrated in this repo:

- Build status notifications (events → message) backed by `flows/events_to_message/build_status_notifications.ygtc`.
- Chat-driven Repo Assistant (messaging with optional event emission) backed by `flows/chat_driven/repo_assistant.ygtc`.

It is intentionally light-weight and references local/demo providers (`local-events`, `local-messaging`, `demo.bridge.events_to_message`, `demo.worker.repo_assistant`). Swap those IDs in your runner config to target real providers.

Components

- `components/demo_bridge_events_to_message.wasm` – placeholder artifact for the bridge component id.
- `components/demo_worker_repo_assistant.wasm` – placeholder artifact for the worker component id.

Scenarios & golden transcripts:

- `scenarios/build_status_notification.json` ↔ `golden/build_status_notification.json`
- `scenarios/repo_assistant_chat.json` ↔ `golden/repo_assistant_chat.json`

Flows:

- `../../flows/events_to_message/build_status_notifications.ygtc`
- `../../flows/chat_driven/repo_assistant.ygtc`
