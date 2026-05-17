"""ChromaDB-backed semantic index per workspace."""

from __future__ import annotations

import hashlib
import os
from pathlib import Path
from typing import Any


def _data_dir() -> Path:
    if p := os.getenv("NEXUS_DATA_DIR"):
        return Path(p)
    if os.name == "nt":
        appdata = os.environ.get("APPDATA")
        if appdata:
            return Path(appdata) / "nexus-ide"
    return Path(os.environ.get("HOME") or ".") / ".local" / "share" / "nexus-ide"


def _collection_name(workspace_root: str) -> str:
    h = hashlib.sha256(workspace_root.encode()).hexdigest()[:16]
    return f"ws_{h}"


class ChromaStore:
    def __init__(self, chroma_path: Path | None = None) -> None:
        self.chroma_path = chroma_path or (_data_dir() / "chroma")
        self.chroma_path.mkdir(parents=True, exist_ok=True)
        self._client = None

    def _client_lazy(self):
        if self._client is None:
            import chromadb

            self._client = chromadb.PersistentClient(path=str(self.chroma_path))
        return self._client

    def _collection(self, workspace_root: str):
        return self._client_lazy().get_or_create_collection(
            name=_collection_name(workspace_root),
            metadata={"workspace": workspace_root},
        )

    def index_workspace(
        self,
        workspace_root: str,
        *,
        max_files: int = 500,
        chunk_size: int = 800,
    ) -> dict[str, Any]:
        root = Path(workspace_root).resolve()
        coll = self._collection(str(root))
        ids: list[str] = []
        docs: list[str] = []
        metas: list[dict[str, Any]] = []
        count = 0
        for dirpath, dirnames, filenames in os.walk(root):
            dirnames[:] = [
                d
                for d in dirnames
                if d not in {".git", "node_modules", "target", ".venv", "dist"}
            ]
            for name in filenames:
                if count >= max_files:
                    break
                path = Path(dirpath) / name
                if path.suffix.lower() in {
                    ".png",
                    ".jpg",
                    ".gif",
                    ".woff",
                    ".exe",
                    ".dll",
                    ".zip",
                }:
                    continue
                try:
                    text = path.read_text(encoding="utf-8", errors="ignore")
                except OSError:
                    continue
                if not text.strip():
                    continue
                rel = path.relative_to(root).as_posix()
                for i in range(0, len(text), chunk_size):
                    chunk = text[i : i + chunk_size]
                    cid = hashlib.sha256(f"{rel}:{i}".encode()).hexdigest()[:24]
                    ids.append(cid)
                    docs.append(chunk)
                    metas.append({"path": rel, "offset": i})
                count += 1
            if count >= max_files:
                break
        if ids:
            coll.upsert(ids=ids, documents=docs, metadatas=metas)
        return {"files_indexed": count, "chunks": len(ids)}

    def search(
        self, workspace_root: str, query: str, *, k: int = 12
    ) -> list[dict[str, Any]]:
        root = str(Path(workspace_root).resolve())
        coll = self._collection(root)
        if coll.count() == 0:
            return []
        res = coll.query(query_texts=[query], n_results=min(k, max(coll.count(), 1)))
        out: list[dict[str, Any]] = []
        docs = (res.get("documents") or [[]])[0]
        metas = (res.get("metadatas") or [[]])[0]
        dists = (res.get("distances") or [[]])[0]
        for doc, meta, dist in zip(docs, metas, dists, strict=False):
            out.append(
                {
                    "path": (meta or {}).get("path", "?"),
                    "snippet": (doc or "")[:400],
                    "score": float(dist) if dist is not None else 0.0,
                }
            )
        return out
