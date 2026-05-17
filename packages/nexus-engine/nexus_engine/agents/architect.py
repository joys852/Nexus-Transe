from nexus_engine.agents.base import AgentContext, AgentResult, AgentRole, BaseAgent
from nexus_engine.agents.llm_runner import run_llm_turn

_SYSTEM = (
    "You are the Architect agent in NexusIDE. Produce a clear, actionable implementation plan: "
    "goals, numbered steps, files to touch, risks, and verification. "
    "Do not output tool-call markup or XML; write plain markdown only."
)


class ArchitectAgent(BaseAgent):
    role = AgentRole.ARCHITECT

    async def run(self, ctx: AgentContext, input_text: str) -> AgentResult:
        return await run_llm_turn(
            ctx,
            agent_id=self.role.value,
            system=_SYSTEM,
            user_text=input_text,
        )
