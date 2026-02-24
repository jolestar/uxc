# CLI Output Unification Plan

Date: 2026-02-24  
Status: Draft

## 背景

当前 CLI 在输出与参数解析上仍存在分叉：

- endpoint 命令（`list/describe/help/inspect/call`）部分支持 `--format/--text`。
- `cache/auth` 仍为直接文本输出，不走统一 JSON envelope。
- `uxc help` 固定文本输出，和“默认 JSON”原则不一致。
- 动态路由下全局参数存在位置敏感（如 `<op> help --text`）。

## 决策

1. UXC 保持 JSON-first：默认输出 JSON envelope。
2. `uxc help` 默认输出 JSON（`kind=global_help`）。
3. `uxc`（无参数）等价于 `uxc help`，默认输出 JSON。
4. `--text`（或 `--format text`）对所有命令生效，用于人类可读渲染。

## 目标

1. 所有命令共享同一输出协议（成功/失败结构一致）。
2. 所有命令共享同一格式切换行为（json/text）。
3. 消除参数位置敏感，保证动态语法一致性。
4. 兼容自动化场景（稳定字段）和人工调试场景（文本渲染）。

## 架构方案

### 1) 统一语义模型（Invocation）

将 CLI 解析结果归一为单一语义层：

- `GlobalHelp`
- `Version`
- `Cache*`
- `Auth*`
- `Endpoint*`（host help/list/describe/inspect/execute）

### 2) 两阶段解析

1. Stage A：提取全局选项（`--format/--text/profile/cache`），位置无关。
2. Stage B：解析剩余 token 为 `Invocation`（含动态 `<url> <op> help`）。

### 3) 统一输出管线

所有 handler 返回 `Result<OutputEnvelope>`，不直接 `println!`。  
在一个出口根据 `OutputMode` 渲染：

- JSON：直接输出 envelope
- Text：`render_text(envelope)` 按 `kind` 渲染

错误路径同样先转 envelope，再走同一渲染器。

## 语义规范（关键行为）

- `uxc help` -> `kind=global_help`（默认 JSON）
- `uxc --text help` -> 文本帮助
- `uxc <url> help` -> `kind=host_help`
- `uxc <url> <operation> help` -> `kind=operation_detail`
- `uxc cache ...` / `uxc auth ...` -> 也返回 envelope（默认 JSON）

## 实施顺序

1. 输出统一：`cache/auth` + error path 改为 envelope。
2. 两阶段解析：移除 argv 扫描式格式判断，消除位置敏感。
3. Help/Version 统一：`uxc` 与 `uxc help` 进入统一语义命令。
4. 文档与测试：README、CLI 集成测试、快照测试更新。

## 验收标准

1. 所有命令在默认情况下输出合法 JSON envelope。
2. 所有命令在 `--text` 下输出人类可读文本。
3. `--text` 放在命令任意位置语义一致。
4. `uxc` 与 `uxc help` 行为一致（仅输出格式受 `--text` 影响）。
