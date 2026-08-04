# TUI 差距分析与实现路线

> 结论文档。原始调研(带证据标注)见 [claude-code-tui-research.md](./claude-code-tui-research.md)。
> 目标:把 Claude Code 终端 TUI 的视觉元素逐项对照我们的实现,给出差距与实现顺序。

## 对照总表

| Claude Code 视觉元素 | 我们 | 备注 |
|---|---|---|
| Transcript(用户/助手/系统消息) | ✅ chat 组件 | 缺时间戳、`You:` 着色 |
| 工具调用行 + 展开/折叠 | ✅ Activity::Tool | 缺 `⏺`/`⎿` 层级符号与 `(ctrl+o to expand)` 摘要尾 |
| Thinking 块 | ✅ Activity::Thinking | 缺时间戳与分隔线 chrome |
| Todo 清单 | ✅ Activity::Todo(单条演进) | 缺 Ctrl+T 可开关面板 |
| Sub-agent 委派行 + 嵌套 transcript | ✅ Activity::SubAgent | 缺 `name · desc · tokens` 状态面板与 8 色身份 |
| Diff(+绿 −红) | ✅ Activity::Diff | 缺独立 `/diff` 文件列表视图 |
| 图片(kitty 图形协议) | ✅ | 缺剪贴板粘贴 `[Image #N]` chip |
| **层级符号**:`⏺` 圆点 / `⎿` 结果连接 / `├─└─` 树枝 | ❌ | 辨识度最高,纯视觉 |
| **折叠摘要尾**:`… +6 lines (ctrl+o to expand)` / `Called slack 3 times` | ❌ | count-based + affordance |
| **Footer 徽标**:`⏵⏵ accept edits on` / `← for agents` / `PR #446` | ❌ | 状态栏信息密度 |
| **权限确认框**:编号选项 + `❯` 选中 + Esc | ❌ | 模态交互,可用我们的 diff 渲染 |
| **`/` 命令菜单 / `@` 路径补全 / `:` emoji** | ❌ | 输入浮层 |
| **`?` 帮助面板** | ❌ | |
| **Agent 总览屏**:Pinned/Ready/Needs input/Working/Completed 分组 | ❌ | 独立组件 |
| **状态栏模块**:模型 · context% · 花费 · 会话时长 · git 分支 | ❌ | 扩展 App 状态栏 |
| **语义色 token**:claude/text/inactive/suggestion/permission + success/error/warning/merged | ⚠️ 部分 | theme.rs 需 token 化 |
| 错误卡片 / 模式指示器 / 中断指示 / 滚动条 / 多行输入 / suggestion 灰字 | ❌ | 第二梯队 |
| 代码语法高亮 | ❌ | syntect 无资产即可 |

## 实现路线(按顺序)

1. **层级符号 + 折叠摘要尾**(纯视觉,收益最大)
   - `⏺` 主活动前缀、`⎿` 结果缩进连接、`├─/└─` subagent 批次树枝
   - header 追加 count 摘要:`· +6 lines`、`· 15 tool uses · 23.0k tokens`、`(ctrl+o to expand)` affordance
   - Agent View 状态图标(`✻/✽/✢/∙`)进 subagent header
2. **Footer 徽标系统**:模式指示(`⏵⏵`/`⏸`)、`← for agents`、PR 徽标(下划线颜色编码)
3. **权限确认框**:`ActivityKind::Permission` 或模态组件,编号选项 + `❯` 选中,内容复用 diff 渲染
4. **`/` 命令菜单 + `?` 帮助面板**:输入浮层,过滤 + 鼠标点击
5. **Agent 总览屏**:独立组件,分组状态表(复用 activity 状态模型)
6. **语义色 token 化**:theme.rs 收敛为 token 函数 + 主题切换

## 进展

- [x] **① 层级符号 + 折叠摘要尾**:`⏺` 主活动圆点(Tool/SubAgent/Diff)、`⎿` 结果连接符(首行内容)、`├─/└─` 嵌套树枝 + `│` 延续、折叠摘要尾 `… +N lines (click to expand)` / `N steps (click to expand)` / diff `(click to expand)`
- [ ] ② Footer 徽标系统
- [ ] ③ 权限确认框
- [ ] ④ `/` 命令菜单 + `?` 帮助面板
- [ ] ⑤ Agent 总览屏
- [ ] ⑥ 语义色 token 化

## 验收锚点

- 每个视觉元素对照报告中的【文档事实】条目实现,不确定处标注
- 层级符号/折叠尾必须有端到端测试(沿用 tests/host.rs 的 TestBackend 路径)
- 顺序实现,每步可独立提交
