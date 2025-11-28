# Greentic Integration Examples

This repo now includes two runnable integration examples that wire events, messaging, and the Repo Assistant worker using mock/local components:

- Build status notifications: `flows/events_to_message/build_status_notifications.ygtc`
- Chat-driven Repo Assistant: `flows/chat_driven/repo_assistant.ygtc`

Use the per-example guides for details:

- [events_to_message_example.md](events_to_message_example.md)
- [chat_driven_repo_assistant.md](chat_driven_repo_assistant.md)
- [payload_samples.md](payload_samples.md)

Pack fixture:

- `packs/integration-demos/pack.json` pairs the flows with simple scenarios and golden transcripts.

Helper scripts:

- `scripts/run_build_status_demo.sh` – run the build-status notification flow with the demo config.
- `scripts/run_repo_assistant_demo.sh` – run the chat-driven Repo Assistant flow with the demo config.

Demo config:

- `configs/demo_local.yaml` – mock/local provider bindings for the example flows. Override via `CONFIG=/path/to/your.yaml` when running the scripts.
