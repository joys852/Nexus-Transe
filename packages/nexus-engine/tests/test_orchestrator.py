import pytest

from nexus_engine.agents.base import AgentContext
from nexus_engine.agents.orchestrator import MultiAgentOrchestrator


@pytest.mark.asyncio
async def test_pipeline_runs_four_phases():
    orch = MultiAgentOrchestrator()
    ctx = AgentContext(session_id="test", workspace_root=".")
    results = await orch.run_pipeline(ctx, "build a hello world API")
    assert len(results) == 4
    assert "architect" in results[0].metadata.get("agent", "")
