from fastapi.testclient import TestClient

from nexus_engine.api.server import app

client = TestClient(app)


def test_run_task():
    r = client.post(
        "/v1/tasks/run",
        json={
            "session_id": "00000000-0000-0000-0000-000000000001",
            "prompt": "hello",
        },
    )
    assert r.status_code == 200
    data = r.json()
    assert data["status"] == "planning"
