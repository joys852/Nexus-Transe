"""Read API keys from secrets.enc.json (same format as nexus-core SecretVault)."""

from __future__ import annotations

import base64
import json
import os
from pathlib import Path

from cryptography.hazmat.primitives.ciphers.aead import AESGCM


def _data_dir() -> Path:
    if p := os.getenv("NEXUS_DATA_DIR"):
        return Path(p)
    if os.name == "nt":
        appdata = os.environ.get("APPDATA")
        if appdata:
            return Path(appdata) / "nexus-ide"
    home = Path(os.environ.get("HOME") or ".")
    return home / ".local" / "share" / "nexus-ide"


def _master_key() -> bytes:
    raw = os.environ.get("NEXUS_MASTER_KEY")
    if not raw:
        raw = base64.b64encode(bytes(32)).decode("ascii")
    key = base64.b64decode(raw.strip())
    if len(key) != 32:
        raise ValueError("NEXUS_MASTER_KEY must be 32 bytes base64")
    return key


def load_provider_key(provider_id: str, data_dir: Path | None = None) -> str | None:
    root = data_dir or _data_dir()
    path = root / "secrets.enc.json"
    if not path.exists():
        return None
    store = json.loads(path.read_text(encoding="utf-8"))
    enc = (store.get("providers") or {}).get(provider_id)
    if not enc:
        return None
    ciphertext = base64.b64decode(enc["ciphertext_b64"])
    nonce = base64.b64decode(enc["nonce_b64"])
    plain = AESGCM(_master_key()).decrypt(nonce, ciphertext, None)
    return plain.decode("utf-8")
