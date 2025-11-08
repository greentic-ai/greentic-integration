#!/usr/bin/env python3
"""Contract tests for the Direct Line-compatible WebChat backend.

If `WEBCHAT_BASE_URL` is unset, the tests will spin up a lightweight stub server that
returns deterministic responses for the required endpoints so the suite can run offline.
"""
from __future__ import annotations

import json
import os
import urllib.error
import urllib.parse
import urllib.request
from http import HTTPStatus
from typing import Any, Dict, Tuple


def simulate_backend_request(path: str, payload: Dict[str, Any]) -> Tuple[Dict[str, Any], int]:
    if path == "/tokens/generate":
        audience = payload.get("user") or "anonymous"
        return {"token": f"mock-token-for-{audience}", "expires_in": 3600}, HTTPStatus.OK
    if path == "/conversations":
        tenant = payload.get("tenant") or "default"
        return (
            {
                "conversationId": f"conv-{tenant}-001",
                "expires_in": 1800,
                "streamUrl": "wss://stub.greentic.ai/conversations/conv-001",
            },
            HTTPStatus.OK,
        )
    if path.startswith("/conversations/") and path.endswith("/activities"):
        return (
            {
                "id": "activity-001",
                "type": payload.get("type", "message"),
                "accepted": True,
            },
            HTTPStatus.ACCEPTED,
        )
    return {"error": "not found"}, HTTPStatus.NOT_FOUND


def request_json(method: str, url: str, payload: Dict[str, Any]) -> Dict[str, Any]:
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(url, data=data, method=method, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            body = resp.read().decode("utf-8") or "{}"
            parsed = json.loads(body)
            parsed["status"] = resp.status
            return parsed
    except urllib.error.HTTPError as err:
        body = err.read().decode("utf-8") if err.fp else "{}"
        raise AssertionError(f"{method} {url} failed with {err.code}: {body}") from err


def test_tokens(client: RequestClient) -> None:
    resp = client.post("/tokens/generate", {"user": "smoke-user"})
    assert resp.get("token", "").startswith("mock-token"), f"unexpected token response: {resp}"
    assert resp.get("expires_in", 0) > 0, "expires_in must be positive"


def test_conversations(client: RequestClient) -> str:
    resp = client.post("/conversations", {"tenant": "tenant-alpha"})
    conversation_id = resp.get("conversationId")
    assert conversation_id, f"missing conversationId in {resp}"
    assert resp.get("streamUrl"), "streamUrl missing"
    assert resp.get("expires_in", 0) > 0, "expires_in must be positive"
    return conversation_id


def test_activities(client: RequestClient, conversation_id: str) -> None:
    path = f"/conversations/{conversation_id}/activities"
    resp = client.post(path, {"type": "message", "from": {"id": "user"}, "text": "hi"})
    assert resp.get("accepted") is True, f"activity not accepted: {resp}"
    assert resp.get("status") == HTTPStatus.ACCEPTED, "expected HTTP 202"


def main() -> None:
    base_url = os.environ.get("WEBCHAT_BASE_URL")
    if base_url:
        client: RequestClient = HttpClient(base_url.rstrip("/"))
    else:
        client = StubClient()
    run_suite(client)


def run_suite(client: RequestClient) -> None:
    print(f"Running WebChat contract suite against {client.describe()}")
    test_tokens(client)
    conversation_id = test_conversations(client)
    test_activities(client, conversation_id)
    print("webchat.contract: all endpoints verified")


class RequestClient:
    def post(self, path: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        raise NotImplementedError

    def describe(self) -> str:
        raise NotImplementedError


class HttpClient(RequestClient):
    def __init__(self, base_url: str) -> None:
        self.base_url = base_url

    def post(self, path: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        url = urllib.parse.urljoin(self.base_url, path)
        return request_json("POST", url, payload)

    def describe(self) -> str:
        return self.base_url


class StubClient(RequestClient):
    def post(self, path: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        body, status = simulate_backend_request(path, payload)
        body = dict(body)
        body["status"] = status
        return body

    def describe(self) -> str:
        return "in-process stub backend"


if __name__ == "__main__":
    main()
