from nexus_engine.mcp.stdio_transport import _encode_message, _try_parse_buffer


def test_content_length_roundtrip():
    payload = {"jsonrpc": "2.0", "id": "1", "result": {"ok": True}}
    wire = _encode_message(payload)
    msgs, rest = _try_parse_buffer(wire)
    assert rest == b""
    assert len(msgs) == 1
    assert msgs[0]["id"] == "1"
