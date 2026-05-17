"""ReAct main loop with pause/resume via LangGraph checkpoints."""

from enum import Enum
from typing import TypedDict

from langgraph.checkpoint.memory import MemorySaver
from langgraph.graph import END, StateGraph
from pydantic import BaseModel


class TaskStatus(str, Enum):
    IDLE = "idle"
    PLANNING = "planning"
    ACTING = "acting"
    OBSERVING = "observing"
    PAUSED = "paused"
    DONE = "done"
    FAILED = "failed"


class GraphState(TypedDict):
    session_id: str
    status: str
    messages: list[dict]
    pending_tool_calls: list[dict]
    last_observation: str | None


class RunTaskInput(BaseModel):
    session_id: str
    prompt: str
    model_id: str | None = None
    agent_profile: str = "default"


def build_react_graph() -> StateGraph:
    """Minimal ReAct graph — expand with multi-agent nodes in Beta."""

    def plan(state: GraphState) -> GraphState:
        state = dict(state)
        state["status"] = TaskStatus.PLANNING.value
        return state

    def act(state: GraphState) -> GraphState:
        state = dict(state)
        state["status"] = TaskStatus.ACTING.value
        return state

    def observe(state: GraphState) -> GraphState:
        state = dict(state)
        state["status"] = TaskStatus.OBSERVING.value
        state["last_observation"] = "stub"
        return state

    def should_continue(state: GraphState) -> str:
        if state.get("status") == TaskStatus.DONE.value:
            return END
        return "act"

    graph = StateGraph(GraphState)
    graph.add_node("plan", plan)
    graph.add_node("act", act)
    graph.add_node("observe", observe)
    graph.set_entry_point("plan")
    graph.add_edge("plan", "act")
    graph.add_edge("act", "observe")
    graph.add_conditional_edges("observe", should_continue, {"act": "act", END: END})
    return graph


def compile_graph():
    memory = MemorySaver()
    return build_react_graph().compile(checkpointer=memory)
