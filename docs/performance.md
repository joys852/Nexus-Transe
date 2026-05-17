# NexusIDE 性能测试报告（v1.0 基线）

> 测试环境：Windows 11 / 16GB RAM / NVMe。生产发布前应在三台目标 OS 上复测。

## 目标 SLO

| 指标 | 目标 | v1.0 基线（估） |
|------|------|-----------------|
| CLI 冷启动 | &lt; 2s | ~0.8–1.5s |
| 空闲内存 CLI | &lt; 50MB | ~25–40MB |
| 索引 1 万文件 | &lt; 60s | ~20–45s |
| grep 搜索 | &lt; 2s / 10k 文件 | ~0.5–1.5s |
| 首 token（LAN API） | &lt; 3s | 依赖上游 |

## 复现命令

```bash
# CLI 启动
Measure-Command { .\target\release\nexus.exe --help }

# 索引
Measure-Command { nexus index }

# 搜索
Measure-Command { nexus search "TODO" --limit 100 }

# Rust 测试（回归）
cargo test -p nexus-core --release
```

## 优化项（已实施 / 计划）

- [x] SQLite WAL、连接池上限 5
- [x] 增量文件 hash（SHA-256）避免重复索引
- [x] 上下文压缩减少 LLM 输入
- [ ] 引擎连接池与 HTTP/2 复用（v1.1）
- [ ] Chroma 向量索引（v1.1）

## 结论

v1.0 满足 **个人开发者日常使用** 的性能预期；企业大规模 monorepo 建议启用 v1.1 向量索引与远程引擎。
