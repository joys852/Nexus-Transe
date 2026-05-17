from nexus_engine.agents.base import AgentContext, AgentResult, AgentRole, BaseAgent
from nexus_engine.agents.llm_runner import run_llm_turn

_SYSTEM = (
    "You are the Review agent in NexusIDE. Critique the implementation: bugs, security, style, "
    "missing tests. Be concise with bullet findings. No tool-call markup."
)


class ReviewAgent(BaseAgent):
    role = AgentRole.REVIEW

    async def run(self, ctx: AgentContext, input_text: str) -> AgentResult:
        return await run_llm_turn(
            ctx,
            agent_id=self.role.value,
            system=_SYSTEM,
            user_text=input_text,
        )
