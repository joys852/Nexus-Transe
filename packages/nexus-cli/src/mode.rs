#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatMode {
    #[default]
    Default,
    /// Plan first — no tool calls until user switches back.
    Plan,
    /// Multi-agent pipeline (architect → code → review → test).
    Agent,
}

impl ChatMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Plan => "plan",
            Self::Agent => "agent",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "chat",
            Self::Plan => "plan",
            Self::Agent => "agent",
        }
    }

    pub fn from_slash(cmd: &str) -> Option<Self> {
        match cmd {
            "/chat" | "/default" => Some(Self::Default),
            "/plan" => Some(Self::Plan),
            "/agent" => Some(Self::Agent),
            _ => None,
        }
    }
}
