# 目录结构

```
nexus-ide/
├── .github/workflows/          # CI/CD
├── docs/                       # 架构、API、开发指南
├── migrations/                 # SQLite 迁移 (sqlx)
├── packages/
│   ├── shared/                 # 协议与类型
│   │   ├── proto/              # Protobuf / JSON Schema
│   │   └── typescript/         # TS 类型（与 HTTP API 对齐）
│   ├── nexus-core/             # Rust 核心库
│   │   └── src/
│   │       ├── config/
│   │       ├── storage/        # SQLite + 迁移
│   │       ├── tools/          # 工具实现 + 沙箱
│   │       ├── sync/           # IPC / 事件
│   │       ├── index/          # Tree-sitter 索引
│   │       └── models/
│   ├── nexus-cli/              # CLI 二进制
│   │   └── src/
│   └── nexus-engine/            # Python LangGraph 引擎
│       ├── nexus_engine/
│       │   ├── agents/
│       │   ├── graph/
│       │   ├── tools/
│       │   ├── context/
│       │   ├── llm/
│       │   └── api/
│       └── tests/
├── Cargo.toml                  # Rust workspace
├── package.json                # pnpm workspace（shared/typescript）
├── pyproject.toml              # uv / Python workspace
└── README.md
```

## 模块依赖关系

```
nexus-cli ──► nexus-core ◄──► SQLite / FS / Tree-sitter
                  │
                  ▼ HTTP + 工具回调
            nexus-engine (Python)
                  │
                  └──► ChromaDB (via nexus-core RPC 或直连)
```
