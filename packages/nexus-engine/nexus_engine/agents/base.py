from abc import ABC, abstractmethod
from enum import Enum

from pydantic import BaseModel, Field


class AgentRole(str, Enum):
    ARCHITECT = "architect"
    CODE = "code"
    REVIEW = "review"
    TEST = "test"


class AgentContext(BaseModel):
    session_id: str
    workspace_root: str | None = None
    model_id: str | None = None
    messages: list[dict] = Field(default_factory=list)
    max_agent_chars: int = 6_000


class AgentResult(BaseModel):
    content: str
    tool_calls: list[dict] = Field(default_factory=list)
    metadata: dict = Field(default_factory=dict)


class BaseAgent(ABC):
    role: AgentRole

    @abstractmethod
    async def run(self, ctx: AgentContext, input_text: str) -> AgentResult: ...
