from nexus_engine.agents.base import AgentContext, AgentResult, AgentRole, BaseAgent
from nexus_engine.agents.llm_runner import run_llm_turn

_SYSTEM = (
    "You are the Code agent in NexusIDE. Implement or sketch the change described in the plan: "
    "concrete file paths, code snippets, and commands. Plain text/markdown only—no tool-call XML."
)


class CodeAgent(BaseAgent):
    role = AgentRole.CODE

    async def run(self, ctx: AgentContext, input_text: str) -> AgentResult:
        return await run_llm_turn(
            ctx,
            agent_id=self.role.value,
            system=_SYSTEM,
            user_text=input_text,
        )
