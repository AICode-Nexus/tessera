# Reasonix Lessons For Tessera

日期：2026-05-15

来源：

- 官方仓库：<https://github.com/esengine/DeepSeek-Reasonix>
- 官方架构文档：<https://github.com/esengine/DeepSeek-Reasonix/blob/main/docs/ARCHITECTURE.md>
- 真实缓存案例：<https://github.com/esengine/DeepSeek-Reasonix/blob/main/benchmarks/real-world-cache/README.md>
- `v0.43.0` release note：<https://github.com/esengine/DeepSeek-Reasonix/releases/tag/v0.43.0>

复核时的官方状态：

- `main` commit：`43cae05`
- npm 包：`reasonix@0.43.0`
- Node 要求：`>=22`
- license：MIT
- GitHub API 快照：2813 stars、152 forks、71 open issues

这些数字是 2026-05-15 的时间点快照，只用于本轮设计复核，不作为长期文档里的静态产品事实。

## 1. 目的

本文把 Reasonix 官方仓库中的可吸收设计转化为 Tessera 的架构输入。

吸收原则：

- 学工程约束，不复制 DeepSeek-only 产品边界。
- 保持 Tessera model-agnostic，provider-specific 能力只能进入 capability、route decision 或安全 extension。
- 不因为 Reasonix 已实现 coding agent，就扩大 Tessera v0.1 范围。
- 所有采纳项必须能落到明确阶段和验证方式。

## 2. 值得吸收的设计

### 2.1 Cache-stable context

Reasonix 的核心不是“打开了 DeepSeek prefix cache”，而是把 cache stability 作为 loop 不变量：

- `ImmutablePrefix`：system prompt、tool specs、few-shot 在 session 内固定。
- `AppendOnlyLog`：历史只追加，不重排、不原地改写。
- `VolatileScratch`：reasoning、临时计划等 scratch 不回灌到下一次 provider input。
- Auto-compact 以追加 summary 的方式处理大上下文，避免破坏已有前缀。

Tessera 处理方式：

- v0.1：保留 provider capability、usage/cache/cost trace 字段；不实现长期 session context builder。
- v0.2：设计 context workbench 时明确区分 stable prefix、append-only transcript、volatile scratch。
- v0.2：client status 从 `UsageReported` 投影 cache/cost 摘要，TUI/GUI 不直接解析 provider 私有字段。
- v0.3+：任何 context compaction 都必须可追踪，并优先通过追加 summary 或 artifact handle 表达。

### 2.2 Ordered parallel tool dispatch

Reasonix 的 `parallelSafe?: boolean` 是 opt-in：只读工具可以并发执行，但 tool result yield 和 history append 仍按声明顺序落地。第一个 mutating tool 形成 serial barrier，保留 read-after-write 顺序。

Tessera 处理方式：

- v0.1：不执行工具，只预留 tool/policy/trace 命名。
- v0.3：Tool descriptor 必须包含 `parallel_safe`，默认 `false`。
- v0.3：并行调度只允许 policy 认可的只读/隔离工具进入同一 chunk。
- v0.3：即使执行并发，trace append 和模型可见结果必须保持声明顺序。
- v0.3：提供强制串行的排障开关。

### 2.3 Tool-call repair telemetry

Reasonix 针对 DeepSeek 的经验失败模式做了 flatten、scavenge、truncation、storm 四道 repair pass。Tessera 不应把这些 pass 硬编码成 DeepSeek-only 行为，但应把 repair 作为未来 agent loop 的可观测事件。

Tessera 处理方式：

- v0.1：只在文档中预留 `tool_repair_reported`。
- v0.3+：repair report 必须记录修复类型、计数、是否改变执行计划、是否触发 policy/route 变化。
- v0.3+：repair 不应把 provider 原始 reasoning 原文直接写入 trace；必要时进入脱敏 artifact。

### 2.4 Visible cost control

Reasonix 的成本控制包含 flash-first preset、单回合 `/pro`、失败信号升档、turn-end compaction 和可见 cost badge。更重要的是，`v0.43.0` 移除了连续只读自动升 pro 的启发式：官方说明认为这只会让无进展读取循环更贵，正确方向是检测、停止并提示。

Tessera 处理方式：

- v0.1：只记录 usage/cache/cost/route decision，不实现 Auto router。
- v0.2：model router 草案必须记录 route reason 和 cost/capability 输入，不默认开启。
- v0.2：加入 no-progress loop detection 设计，优先 stop / ask / summarize，而不是直接升更贵模型。
- v0.3+：任何模型升档必须对用户可见，并写入 trace。
- v0.3+：没有 usage/cache/cost telemetry 和 no-progress loop policy 前，不上线自动升档。

### 2.5 Subagent 是上下文控制，不是 swarm

Reasonix 官方 non-goal 明确写出：subagent 是 cost-reduction mechanism，不是 multi-agent orchestration primitive。其核心价值是隔离大范围读文件/搜索链，只把最终摘要回到主上下文。

Tessera 处理方式：

- v0.1：Artifact 和 AgentEvent 命名先打底。
- v0.5：single agent loop 先稳定，再做 subagent。
- v0.6：persistent sub-agent sessions 必须用 artifact/context handle，不把完整 transcript 塞回父上下文。
- v0.8：swarm scheduler 只能建立在稳定 agent/task/trace/handoff 之上。

## 3. 纳入路线

### v0.1 保持不扩 scope

- 不实现工具执行。
- 不实现 agent runtime。
- 不实现 MCP runtime。
- 不实现 YOLO / trusted workspace。
- 不实现 Auto router。
- 只强化 docs、trace 命名和 client status projection。

### v0.2 新增或强化

- Context workbench 设计 stable prefix / append-only transcript / volatile scratch。
- Usage/cache/cost/context summary 从 trace 和 live events 聚合。
- Model router 草案必须包含 route reason、cost visibility 和 no-progress loop policy。
- Tauri GUI shell 只消费 `ClientSnapshot`，不得解析 provider extension。

### v0.3-v0.4 新增或强化

- Tool descriptor 增加 `parallel_safe`，默认 false。
- Tool dispatcher 支持 serial barrier 和 ordered result append。
- Repair telemetry 进入 trace。
- No-progress loop detection 先于自动升档。
- MCP tool 通过 Tessera Tool/Policy/Trace 接入，第三方 parallel safety 必须显式声明。

### v0.5+

- Single agent loop 使用 stable context contract。
- Skill runtime 和 subagent 默认受 context budget 限制。
- Structured handoff 只返回 summary、evidence、metrics 和 artifact handles。
- Swarm 不得绕过 route/cost/policy/trace。

## 4. 明确不吸收

- 不把 Tessera 做成 DeepSeek-only。
- 不把 Reasonix 的 TypeScript/Ink 架构作为 Tessera 主 runtime。
- 不把 provider-specific repair pass 写进 core 协议 payload。
- 不默认静默升档。
- 不把 subagent 当作 v0.1/v0.2 的多 agent 协调能力。
