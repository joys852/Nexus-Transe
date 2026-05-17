from nexus_engine.agents.base import AgentContext, AgentResult, AgentRole, BaseAgent
from nexus_engine.agents.llm_runner import run_llm_turn

_SYSTEM = (
    "You are the Test agent in NexusIDE. Propose test cases and commands to verify the change. "
    "Include unit/integration checks. Plain markdown only."
)


class TestAgent(BaseAgent):
    role = AgentRole.TEST

    async def run(self, ctx: AgentContext, input_text: str) -> AgentResult:
        return await run_llm_turn(
            ctx,
            agent_id=self.role.value,
            system=_SYSTEM,
            user_text=input_text,
        )
