"""Multi-provider LLM adapter (OpenAI, Anthropic, DeepSeek, Ollama, compatible APIs)."""

from abc import ABC, abstractmethod
from typing import AsyncIterator

from pydantic import BaseModel, Field


class ChatMessage(BaseModel):
    role: str
    content: str


class LLMConfig(BaseModel):
    provider: str = Field(description="openai | anthropic | deepseek | ollama | openai_compatible")
    model: str
    api_key: str | None = None
    base_url: str | None = None
    temperature: float = 0.2
    max_tokens: int | None = None


class LLMProvider(ABC):
    @abstractmethod
    async def complete(self, messages: list[ChatMessage]) -> str: ...

    @abstractmethod
    async def stream(self, messages: list[ChatMessage]) -> AsyncIterator[str]: ...


def create_provider(config: LLMConfig) -> LLMProvider:
    """Factory — implementations wired in Beta."""
    raise NotImplementedError(f"provider {config.provider} not yet implemented")
