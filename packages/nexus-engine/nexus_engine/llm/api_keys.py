"""Resolve provider API keys (toml, env, inline, encrypted vault)."""

from __future__ import annotations

import os
from typing import Any

from nexus_engine.secrets_vault import load_provider_key


def inline_api_key(value: str) -> str | None:
    v = value.strip()
    if not v:
        return None
    if v.startswith(("sk-", "pk-")):
        return v
    if len(v) >= 24 and " " not in v and not v.isidentifier():
        return v
    return None


def resolve_api_key(profile: dict[str, Any] | None) -> str | None:
    if not profile:
        return None
    if key := profile.get("api_key"):
        if str(key).strip():
            return str(key).strip()
    if env := profile.get("api_key_env"):
        env = str(env)
        if v := os.getenv(env):
            return v
        if inline := inline_api_key(env):
            return inline
    vault_id = profile.get("api_key_vault") or profile.get("id")
    if vault_id:
        if key := load_provider_key(str(vault_id)):
            return key
    for e in ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "GPT_AGENT_API_KEY", "OPENAI_API_KEY", "NEXUS_API_KEY"):
        if v := os.getenv(e):
            return v
    return None


def require_api_key(profile: dict[str, Any] | None) -> str:
    key = resolve_api_key(profile)
    if key:
        return key
    pid = (profile or {}).get("id", "active")
    env = (profile or {}).get("api_key_env", "API_KEY")
    raise ValueError(
        f"API key missing for provider '{pid}'. "
        f"Set environment variable {env}, add api_key in providers.toml, or run:\n"
        f"  nexus secrets set --provider {pid} --key YOUR_KEY\n"
        f"Then: nexus provider doctor"
    )
