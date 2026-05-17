"""Multi-agent orchestration: Architect → Code → Review → Test."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from nexus_engine.agents.architect import ArchitectAgent
from nexus_engine.agents.base import AgentContext, AgentResult
from nexus_engine.agents.code_agent import CodeAgent
from nexus_engine.agents.review import ReviewAgent
from nexus_engine.agents.test_agent import TestAgent
from nexus_engine.agents.graph_pipeline import run_graph_pipeline
from nexus_engine.context.compressor import ContextCompressor


class AgentPhase(str, Enum):
    ARCHITECT = "architect"
    CODE = "code"
    REVIEW = "review"
    TEST = "test"
    DONE = "done"


@dataclass
class OrchestratorState:
    session_id: str
    phase: AgentPhase = AgentPhase.ARCHITECT
    artifacts: dict[str, str] = None  # type: ignore

    def __post_init__(self) -> None:
        if self.artifacts is None:
            self.artifacts = {}


class MultiAgentOrchestrator:
    def __init__(self) -> None:
        self.architect = ArchitectAgent()
        self.code = CodeAgent()
        self.review = ReviewAgent()
        self.test = TestAgent()
        self.compressor = ContextCompressor()

    async def run_pipeline(self, ctx: AgentContext, user_goal: str) -> list[AgentResult]:
        """LangGraph pipeline with review→code retry (ROADMAP v2 §3.1)."""
        try:
            return await run_graph_pipeline(ctx, user_goal)
        except Exception:
            return await self._run_pipeline_linear(ctx, user_goal)

    async def _run_pipeline_linear(self, ctx: AgentContext, user_goal: str) -> list[AgentResult]:
        """Fallback linear pipeline if LangGraph unavailable."""
        results: list[AgentResult] = []
        state = OrchestratorState(session_id=ctx.session_id)
        phases = [
            (AgentPhase.ARCHITECT, self.architect),
            (AgentPhase.CODE, self.code),
            (AgentPhase.REVIEW, self.review),
            (AgentPhase.TEST, self.test),
        ]
        prior = user_goal
        for phase, agent in phases:
            state.phase = phase
            input_text = prior if phase == AgentPhase.ARCHITECT else results[-1].content
            result = await agent.run(ctx, input_text)
            results.append(result)
            state.artifacts[phase.value] = result.content
            prior = result.content
            ctx.messages = self.compressor.compress_messages(ctx.messages)
        state.phase = AgentPhase.DONE
        return results
