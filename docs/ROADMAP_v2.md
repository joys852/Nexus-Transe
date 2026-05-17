# NexusIDE CLI v2.0 全面对标计划

> 目标：成为 Rust/Python 生态中最优雅的 AI 编程 CLI，对标 ripgrep/fd/bun/gh 的 UX 水准

### 实施状态（2026-05-16）

| 模块 | 状态 | 路径 |
|------|------|------|
| 错误码 + 美化框 | ✅ | `nexus-core/error_present.rs`, `nexus-cli/errors.rs` |
| 配置验证 + 合并加载 | ✅ | `nexus-core/config.rs` → `load_merged()` |
| SQLite WAL + 连接池 | ✅ | `nexus-core/storage/sqlite.rs` |
| MCP stdio 传输 | ✅ 骨架 | `nexus-core/mcp/stdio_transport.rs`, `nexus-engine/mcp/stdio_transport.py` |
| Markdown 终端渲染 | ✅ 基础 | `nexus-cli/markdown.rs` |
| 多阶段进度条 | ✅ | `nexus-cli/progress.rs` + agent pipeline |
| LangGraph 流水线 | ✅ 含回退 | `nexus-engine/agents/graph_pipeline.py` |
| 分层记忆 | ✅ | `nexus-engine/context/hierarchical_memory.py` |
| SSE status/progress | ✅ 部分 | `chat.py` + `nexus-cli/chat.rs` |
| MCP → 聊天工具链 | ✅ | `mcp/registry.py` + `/v1/mcp/call` + CLI 路由 |
| 工具结果格式化 | ✅ 基础 | `nexus-cli/tool_format.rs` |
| CLI 活动条 / 分隔线对齐 | ✅ | `session_ui.rs` 单行 Working + 统一 `TURN_W` |
| TUI 会话浏览器 | ✅ | `session_tui.rs` · `/sessions` |
| 主题 light/dark/carbon | ✅ | `theme.rs` · `/theme` |
| 协作指示器 | ✅ | `collab.rs` · `.nexus/workspace.lock` · `/collab` |
| 插件市场 TUI | ✅ | `plugin_tui.rs` · `/plugins` |
| 性能分析 | ✅ | `profiler.rs` · `/profile` |
| Markdown 表格 / 有序列表 | ✅ | `markdown.rs` |
| Diff 高亮 | ✅ | `diff.rs` + git_diff / edit_file |
| 完整 Markdown / Diff / TUI | ⏳ | v1.3+ |

## 一、对标分析：顶尖 CLI 的核心特质

| 工具 | UX 亮点 | Nexus 差距 |
|------|---------|------------|
| **ripgrep** | 智能过滤、彩色分组、进度感 | 搜索输出无分组高亮 |
| **fd** | 极简语法、类型图标、实时预览 | 文件列表纯文本 |
| **bun** | 现代化 Spinner、emoji 语义、清晰分层 | 进度指示单调 |
| **gh** | 交互式表格、智能提示、错误恢复 | 表格手工拼接、错误简陋 |
| **fzf** | 模糊搜索、实时预览、键盘驱动 | 无交互式选择器 |
| **docker** | 结构化输出、资源统计、健康检查 | 状态信息分散 |

---

## 二、架构层改进（Foundation）

### 2.1 错误处理系统重构 [P0]

**现状**：`NexusError` 只有描述，用户难以排查

**目标**：错误码体系 + 上下文链 + 智能建议

```rust
// 新的错误体系
#[derive(Debug, Error)]
#[error("[{code}] {message}\n\n{context}\n\n{suggestion}")]
pub struct NexusError {
    code: ErrorCode,      // E001, E002...
    message: String,
    context: Vec<String>, // anyhow::Context 链
    suggestion: Option<String>,
    docs_url: Option<String>,
}

// 使用示例
.read_to_string(&path)
.with_context(|| format!("failed to read project config: {}", path.display()))
.suggestion("Run `nexus init` to create PROJECT.md")
.docs(ErrorCode::E001)
```

**输出示例**：
```
┌─ Error [E042] ─────────────────────────────────────────────┐
│ Database connection failed: database is locked             │
│                                                            │
│ Context:                                                   │
│   1. while loading session history                         │
│   2. while executing `nexus session list`                  │
│                                                            │
│ Suggestion:                                                │
│   • Check if another Nexus process is running              │
│   • Run: lsof | grep nexus.db                              │
│   • Or: nexus db unlock --force                            │
│                                                            │
│ Docs: https://nexuside.dev/docs/errors/E042                │
└────────────────────────────────────────────────────────────┘
```

### 2.2 配置系统统一 [P0]

**现状**：`load_config()` 混合 4 种来源，无验证

**目标**：分层配置 + 前置验证 + 自动补全

```rust
use config::{Config, Environment, File};

#[derive(Deserialize, Validate)]
pub struct NexusConfig {
    #[validate(url)]
    engine_url: String,
    
    #[validate(length(min = 1))]
    default_model: String,
    
    #[validate(range(min = 1024, max = 65535))]
    port: u16,
}

// 加载优先级：default < file < env < args
let cfg = Config::builder()
    .add_source(File::with_name("/etc/nexus/default"))
    .add_source(File::with_name("~/.config/nexus/config").required(false))
    .add_source(Environment::with_prefix("NEXUS"))
    .build()?
    .try_deserialize::<NexusConfig>()?
    .validate()?;
```

### 2.3 SQLite 连接池 [P0]

**现状**：单连接，可能成为瓶颈

**改进**：
```rust
// 读写分离 + 连接池
pub struct SqliteStore {
    writer: Pool<Sqlite>,      // 单写
    reader: Pool<Sqlite>,      // 多读
    wal_mode: bool,
}

// 迁移支持回滚
pub trait Migration {
    fn up(&self) -> &'static str;
    fn down(&self) -> &'static str;  // 新增
    fn checksum(&self) -> u64;        // 完整性校验
}
```

---

## 三、智能体引擎层（Engine）

### 3.1 真正的 LangGraph 状态机 [P0]

**现状**：`orchestrator.py` 是简单 for 循环串行

**目标**：条件边 + 回退机制 + 人工介入点

```python
from langgraph.graph import StateGraph, END
from langgraph.checkpoint.memory import MemorySaver

class AgentState(TypedDict):
    messages: list
    phase: Literal["architect", "code", "review", "test", "done"]
    artifacts: dict
    retries: int
    human_feedback: Optional[str]

# 构建状态机
workflow = StateGraph(AgentState)

# 节点
workflow.add_node("architect", architect_node)
workflow.add_node("code", code_node)
workflow.add_node("review", review_node)
workflow.add_node("test", test_node)
workflow.add_node("human", human_in_the_loop)  # 人工介入

# 条件边：Review 失败回退到 Code
workflow.add_conditional_edges(
    "review",
    lambda s: "pass" if s["review_passed"] else "fail",
    {"pass": "test", "fail": "code"}
)

# Review 和 Test 可并行
workflow.add_edge("code", ["review", "test"])

# 中断点：需要人工确认时暂停
workflow.add_interrupt("human")

app = workflow.compile(checkpointer=MemorySaver())
```

### 3.2 分层记忆系统 [P1]

**现状**：`compressor.compress_messages()` 直接替换，信息丢失

**改进**：详细短期 + 摘要长期

```python
@dataclass
class HierarchicalMemory:
    # 最近 10 轮完整对话
    short_term: deque[Message] = field(default_factory=lambda: deque(maxlen=10))
    
    # 压缩后的历史摘要
    long_term: list[str] = field(default_factory=list)
    
    # 关键事实（向量检索）
    facts: ChromaCollection
    
    def retrieve(self, query: str, k: int = 5) -> list[str]:
        # 优先短期记忆
        # 然后相关事实
        # 最后长期摘要
```

### 3.3 MCP 全双工实现 [P0]

**现状**：stdio 未完成，阻塞第三方工具生态

```python
class McpStdioTransport:
    """全双工 MCP stdio 传输层"""
    
    def __init__(self, cmd: list[str]):
        self.proc = asyncio.create_subprocess_exec(
            *cmd,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        self.pending: dict[str, asyncio.Future] = {}
        self._reader_task: Optional[asyncio.Task] = None
    
    async def start(self):
        self._reader_task = asyncio.create_task(self._read_loop())
    
    async def _read_loop(self):
        """后台读取 stdout，分发给 pending futures"""
        while True:
            line = await self.proc.stdout.readline()
            msg = json.loads(line)
            if msg["id"] in self.pending:
                self.pending[msg["id"]].set_result(msg)
    
    async def send_request(self, method: str, params: dict) -> dict:
        """非阻塞请求"""
        msg_id = str(uuid.uuid4())
        future = asyncio.get_event_loop().create_future()
        self.pending[msg_id] = future
        
        await self.proc.stdin.write(json.dumps({
            "jsonrpc": "2.0",
            "id": msg_id,
            "method": method,
            "params": params
        }).encode())
        
        return await asyncio.wait_for(future, timeout=30)
```

---

## 四、输出美化与格式化（UI/UX）

### 4.1 Markdown 富渲染 [P0]

```rust
use termimad::{MadSkin, LineStyle};
use syntect::parsing::SyntaxSet;

pub struct MarkdownRenderer {
    skin: MadSkin,
    syntax_set: SyntaxSet,
    width: usize,
}

impl MarkdownRenderer {
    pub fn render(&self, md: &str) -> String {
        // 标题层级：H1=bold+underline+primary_color
        // 代码块：syntect 语法高亮 + 背景色块
        // 表格：tabled + Unicode border
        // 引用块：左侧边框 + 斜体
        // 列表：自定义 bullet 符号
    }
}
```

**输出效果**：
```
┌─ Response ─────────────────────────────────────────────────┐
│                                                            │
│  I'll help you refactor this function. Here's the plan:    │
│                                                            │
│  ## Issues Found                                           │
│                                                            │
│  1. Error handling uses `unwrap()` (line 42)              │
│  2. Missing documentation                                 │
│  3. Clone-heavy string operations                         │
│                                                            │
│  ## Suggested Changes                                      │
│                                                            │
│  ┌─ src/db.rs ──────────────────────────────────────────┐ │
│  │                                                      │ │
│  │  - let data = fetch().unwrap();                      │ │
│  │  + let data = fetch().context("fetch failed")?;      │ │
│  │                                                      │ │
│  └─ Rust · 1 change ────────────────────────────────────┘ │
│                                                            │
│  Apply these changes? [y/N/a(ll)/d(iff)/s(kip)]            │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### 4.2 智能进度系统 [P1]

```rust
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct NexusProgress {
    multi: MultiProgress,
    stages: HashMap<String, ProgressBar>,
}

impl NexusProgress {
    pub fn new() -> Self {
        // 多阶段进度
        // [1/4] Architect ⠋ analyzing codebase...
        // [2/4] Code      ⠙ editing 3 files...
        // [3/4] Review    ✓ completed
        // [4/4] Test      ⠴ running cargo test...
    }
    
    pub fn set_stage(&mut self, stage: &str, status: StageStatus) {
        match status {
            Running(msg) => self.spinner(stage, msg),
            Progress(pct) => self.bar(stage, pct),
            Done(duration) => self.checkmark(stage, duration),
            Failed(err) => self.cross(stage, err),
        }
    }
}
```

### 4.3 结构化输出协议 [P1]

Engine 返回带类型标注的块：

```typescript
// 内容块类型
type ContentBlock =
  | { type: 'text'; content: string }
  | { type: 'code'; language: string; content: string; file?: string; editable: boolean }
  | { type: 'diff'; original: string; modified: string; path: string }
  | { type: 'table'; headers: string[]; rows: string[][] }
  | { type: 'thinking'; content: string; phase: string }
  | { type: 'file_tree'; root: string; nodes: FileNode[] }
  | { type: 'error'; code: string; message: string; suggestion?: string };

// SSE 事件扩展
type SseEvent =
  | { event: 'token'; delta: string }
  | { event: 'block_start'; block_type: string }
  | { event: 'block_delta'; chunk: string }
  | { event: 'block_end' }
  | { event: 'thinking'; phase: string; content: string }
  | { event: 'progress'; stage: string; percent: number; detail?: string }
  | { event: 'file_change'; path: string; action: 'create' | 'edit' | 'delete'; lines: number }
  | { event: 'tool_call'; call_id: string; tool_name: string; arguments: unknown }
  | { event: 'tool_result'; call_id: string; status: 'ok' | 'error'; output?: unknown; duration_ms: number };
```

### 4.4 交互式组件 [P2]

```rust
use inquire::{MultiSelect, Confirm, Text, Select};

pub struct InteractiveUI;

impl InteractiveUI {
    /// 多选文件
    pub fn select_files(files: &[PathBuf]) -> Vec<&PathBuf> {
        MultiSelect::new("Select files to edit:", files)
            .with_help_message("↑↓ to move, space to select, enter to confirm")
            .prompt()
    }
    
    /// 确认工具调用
    pub fn confirm_tool(tool: &str, args: &str, preview: Option<&str>) -> bool {
        // 显示工具详情 + 影响预览
        // [y]es [n]o [a]lways [s]kip similar
    }
    
    /// 模糊搜索选择
    pub fn fuzzy_select<T: Display>(items: &[T], prompt: &str) -> Option<&T> {
        // fzf 风格实时过滤
    }
    
    /// 实时输入补全
    pub fn input_with_completion(
        prompt: &str,
        completer: impl Fn(&str) -> Vec<String>,
    ) -> String {
        // Tab 补全，类似 fish
    }
}
```

### 4.5 错误恢复与建议 [P1]

```rust
pub trait Recoverable {
    fn try_recover(&self) -> Option<RecoveryAction>;
}

impl Recoverable for NexusError {
    fn try_recover(&self) -> Option<RecoveryAction> {
        match self.code {
            ErrorCode::E042 => Some(RecoveryAction::Suggest(
                "Run `nexus db unlock` to force unlock"
            )),
            ErrorCode::E101 => Some(RecoveryAction::Interactive(
                Box::new(|| interactive_fix_provider_config())
            )),
            _ => None,
        }
    }
}

// 使用
if let Err(e) = result {
    if let Some(action) = e.try_recover() {
        match action.execute() {
            Ok(()) => println!("Fixed! Retrying..."),
            Err(_) => println!("Please fix manually and try again"),
        }
    }
}
```

---

## 五、性能优化

### 5.1 内存优化 [P2]

```rust
// String → Arc<str> 减少克隆
pub struct ProjectContext {
    root: Arc<Path>,              // 共享
    name: Arc<str>,               // 不可变共享
    project_md: Option<Arc<str>>,
}

// 大文件按需加载
pub struct FileContent {
    path: Arc<Path>,
    content: OnceCell<Arc<str>>,  // 懒加载
}
```

### 5.2 异步优化 [P2]

```rust
// async_trait → trait-variant (RPITIT)
pub trait ToolHandler: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    fn execute(&self, request: ToolCallRequest) -> impl Future<Output = Result<ToolCallResult>> + Send;
}

// 避免 Box::pin
```

### 5.3 编译优化 [P3]

```toml
# Cargo.toml
[profile.release]
lto = "thin"
codegen-units = 1
strip = true
opt-level = 3

# 使用 mold linker
[build]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

---

## 六、版本路线图

### v1.1 — Foundation（4 周）

**P0 必做**：
- [x] 错误码体系 + 美化输出
- [x] 配置验证 + 统一加载
- [x] SQLite 连接池 + WAL 优化
- [x] MCP stdio 全双工（传输层；工具发现接入待完成）
- [x] Markdown 基础渲染（代码高亮）

**P1 可选**：
- [x] 工具结果格式化（read_file / glob / MCP 预览）
- [x] 简单进度条

### v1.2 — Intelligence（4 周）

**P0 必做**：
- [x] LangGraph 状态机重构（含 linear 回退）
- [x] 分层记忆系统
- [x] 智能体回退机制
- [x] SSE 事件扩展（thinking/progress）

**P1 可选**：
- [x] 多阶段进度条
- [x] 交互式工具确认

### v1.3 — Polish（3 周）

**P0 必做**：
- [x] 完整 Markdown 渲染（表格/引用/列表）
- [x] Diff 高亮
- [x] 结构化输出协议稳定版（`block_start` / `block_end` SSE 骨架）

**P1 可选**：
- [x] 模糊搜索选择器（TUI 子序列匹配过滤）
- [x] 主题系统（light/dark/carbon）

### v2.0 — Premium（持续）

- [x] 交互式会话浏览器（TUI）
- [x] 实时协作指示器
- [x] 插件市场浏览器
- [x] 性能分析器集成

---

## 七、技术栈选型

| 功能 | Rust | Python |
|------|------|--------|
| Markdown | `termimad` / `comrak` | `rich.markdown` |
| 语法高亮 | `syntect` | `pygments` |
| 进度条 | `indicatif` | `rich.progress` |
| 表格 | `tabled` | `rich.table` |
| 交互提示 | `inquire` | `questionary` |
| 错误处理 | `thiserror` + `anyhow` | `pydantic` |
| 配置 | `config` + `validator` | `pydantic-settings` |
| TUI | `ratatui` | - |

---

## 八、成功指标

| 指标 | 现状 | v1.1 目标 | v2.0 目标 |
|------|------|-----------|-----------|
| 错误可读性 | ⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| 输出美观度 | ⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| 响应速度 | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| 内存占用 | ⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| 用户满意度 | - | 4.0/5 | 4.8/5 |

---

## 九、实施建议

1. **先做错误处理** — 这是所有体验的基础
2. **配置系统次之** — 影响所有功能的稳定性
3. **并行开发 MCP** — 阻塞生态的关键路径
4. **输出美化渐进** — 先框架后细节
5. **收集反馈循环** — 每版本发布用户调研

---

**Ready to start?** 建议从错误码体系或 Markdown 渲染开始，这两个模块独立且用户感知最强。
