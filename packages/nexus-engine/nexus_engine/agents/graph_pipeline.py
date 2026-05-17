"""LangGraph multi-agent pipeline with review retry (ROADMAP v2 §3.1)."""

from __future__ import annotations

from typing import Any, Literal, TypedDict

from langgraph.checkpoint.memory import MemorySaver
from langgraph.graph import END, StateGraph

from nexus_engine.agents.architect import ArchitectAgent
from nexus_engine.agents.base import AgentContext, AgentResult
from nexus_engine.agents.code_agent import CodeAgent
from nexus_engine.agents.review import ReviewAgent
from nexus_engine.agents.test_agent import TestAgent
from nexus_engine.context.hierarchical_memory import HierarchicalMemory


class PipelineState(TypedDict, total=False):
    user_goal: str
    ctx: AgentContext
    phase: str
    artifacts: dict[str, str]
    results: list[dict[str, Any]]
    review_passed: bool
    retries: int
    memory_note: str


def _review_passed(content: str) -> bool:
    lower = content.lower()
    blockers = ("critical", "blocker", "must fix", "security issue", "cannot ship")
    return not any(b in lower for b in blockers)


async def _run_agent(agent, ctx: AgentContext, text: str) -> AgentResult:
    return await agent.run(ctx, text)


def build_agent_graph() -> StateGraph:
    architect = ArchitectAgent()
    code = CodeAgent()
    review = ReviewAgent()
    test = TestAgent()

    async def architect_node(state: PipelineState) -> PipelineState:
        ctx = state["ctx"]
        r = await _run_agent(architect, ctx, state["user_goal"])
        arts = dict(state.get("artifacts") or {})
        arts["architect"] = r.content
        return {
            **state,
            "phase": "architect",
            "artifacts": arts,
            "results": (state.get("results") or []) + [_result_dict("architect", r)],
        }

    async def code_node(state: PipelineState) -> PipelineState:
        ctx = state["ctx"]
        prior = state["artifacts"].get("architect", state["user_goal"])
        r = await _run_agent(code, ctx, prior)
        arts = dict(state["artifacts"])
        arts["code"] = r.content
        return {
            **state,
            "phase": "code",
            "artifacts": arts,
            "results": (state.get("results") or []) + [_result_dict("code", r)],
        }

    async def review_node(state: PipelineState) -> PipelineState:
        ctx = state["ctx"]
        prior = state["artifacts"].get("code", "")
        r = await _run_agent(review, ctx, prior)
        passed = _review_passed(r.content)
        arts = dict(state["artifacts"])
        arts["review"] = r.content
        return {
            **state,
            "phase": "review",
            "artifacts": arts,
            "review_passed": passed,
            "results": (state.get("results") or []) + [_result_dict("review", r)],
        }

    async def test_node(state: PipelineState) -> PipelineState:
        ctx = state["ctx"]
        prior = state["artifacts"].get("code", state["user_goal"])
        r = await _run_agent(test, ctx, prior)
        arts = dict(state["artifacts"])
        arts["test"] = r.content
        return {
            **state,
            "phase": "test",
            "artifacts": arts,
            "results": (state.get("results") or []) + [_result_dict("test", r)],
        }

    def route_after_review(state: PipelineState) -> Literal["test", "retry"]:
        if state.get("review_passed"):
            return "test"
        retries = state.get("retries", 0)
        if retries >= 2:
            return "test"
        return "retry"

    async def bump_retry(state: PipelineState) -> PipelineState:
        return {**state, "retries": state.get("retries", 0) + 1}

    g = StateGraph(PipelineState)
    g.add_node("architect", architect_node)
    g.add_node("code", code_node)
    g.add_node("review", review_node)
    g.add_node("test", test_node)
    g.add_node("retry", bump_retry)

    g.set_entry_point("architect")
    g.add_edge("architect", "code")
    g.add_edge("code", "review")
    g.add_conditional_edges(
        "review",
        route_after_review,
        {"test": "test", "retry": "retry"},
    )
    g.add_edge("retry", "code")
    g.add_edge("test", END)
    return g


def _result_dict(agent: str, r: AgentResult) -> dict[str, Any]:
    return {
        "agent": agent,
        "content": r.content,
        "metadata": r.metadata,
    }


_compiled = None


def get_compiled_pipeline():
    global _compiled
    if _compiled is None:
        _compiled = build_agent_graph().compile(checkpointer=MemorySaver())
    return _compiled


async def run_graph_pipeline(ctx: AgentContext, user_goal: str) -> list[AgentResult]:
    """Execute LangGraph pipeline; returns AgentResult list in phase order."""
    memory = HierarchicalMemory()
    memory.add_message({"role": "user", "content": user_goal})

    initial: PipelineState = {
        "user_goal": user_goal,
        "ctx": ctx,
        "artifacts": {},
        "results": [],
        "retries": 0,
        "review_passed": False,
        "memory_note": memory.inject_system_context(),
    }
    app = get_compiled_pipeline()
    config = {"configurable": {"thread_id": ctx.session_id}}
    final = await app.ainvoke(initial, config)
    results = final.get("results") or []
    out: list[AgentResult] = []
    for item in results:
        out.append(
            AgentResult(
                content=item.get("content", ""),
                metadata=item.get("metadata") or {"agent": item.get("agent", "?")},
            )
        )
    return out
