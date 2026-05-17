"""Context manager: incremental index, vector retrieval, AST (Tree-sitter in Beta)."""

from pydantic import BaseModel, Field


class ContextChunk(BaseModel):
    file_path: str
    start_line: int
    end_line: int
    content: str
    score: float = 0.0
    source: str = "keyword"  # keyword | vector | ast


class ContextBudget(BaseModel):
    max_tokens: int = 32_000
    reserved_for_tools: int = 4_000


class ContextManager:
    def __init__(self, workspace_root: str, budget: ContextBudget | None = None) -> None:
        self.workspace_root = workspace_root
        self.budget = budget or ContextBudget()

    async def retrieve(self, query: str, limit: int = 20) -> list[ContextChunk]:
        """Hybrid retrieval — Chroma + Tree-sitter wired in Beta."""
        _ = query
        return []

    async def index_file(self, file_path: str) -> None:
        _ = file_path
