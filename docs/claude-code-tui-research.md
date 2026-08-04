# Claude Code 终端 TUI 可见视觉元素研究

> 调研快照：2026-08-04。范围是 Claude Code 自身绘制的终端界面，包括 classic 与 fullscreen 两种渲染器、主会话、子代理及 Agent View；不把 shell 命令自身的任意输出算作 Claude Code 的固定视觉元素。

## 1 总览

### 1.1 证据口径

本文逐项使用三种标签：

- **【文档事实】**：Anthropic 当前官方文档直接说明的行为或文字，可视为本次调研时的产品契约。
- **【官方仓库观察】**：Claude Code 官方 GitHub 仓库中 issue、截图或终端转录所呈现的具体版本外观。它能证明“这个样子确实出现过”，但不保证当前版本、所有主题或所有终端完全相同。
- **【不确定】**：官方资料没有固定像素、字符或颜色值；本文只说明可确认的结构，不把推断写成事实。

颜色尤其受 Claude Code 主题、终端 16/256/true-color 能力及用户调色板影响。官方主题契约定义的是语义 token，而不是一组跨终端恒定的 RGB 值。[主题与终端配置](https://code.claude.com/docs/en/terminal-config)（访问：2026-08-04）

### 1.2 两种整体渲染形态

**名称：classic 主界面。**  
**外观：**会话输出进入终端原生 scrollback；新输出持续追加时，输入区会随内容向下移动。它不是一个接管整屏、固定分区的应用画布。  
**位置：**整个终端标签页；适合依赖终端原生查找、选择与回滚的工作流。  
**证据：**【文档事实】[Fullscreen mode：与 classic 的差异](https://code.claude.com/docs/en/fullscreen)（访问：2026-08-04）

**名称：fullscreen 主界面。**  
**外观：**使用 alternate screen buffer；输入框固定在底部，Transcript 占据其上方可滚动区域，只渲染可见消息，并支持鼠标。用户消息可有整条背景填充。  
**位置：**整个终端标签页；用 <code>/tui fullscreen</code> 进入，<code>/tui default</code> 恢复默认。2026-05-06 及之后首次使用的新用户默认进入 fullscreen，旧用户保留原设置。  
**证据：**【文档事实】[Fullscreen mode](https://code.claude.com/docs/en/fullscreen)（访问：2026-08-04）

**名称：界面纵向骨架。**  
**外观：**以下是根据官方文档合成的结构示意，不是逐字符截图：

~~~text
启动标识 / 会话标题（出现时）
Transcript：用户、Claude、工具调用与结果
任务 / 子代理状态区（有活动项时）
固定或随输出移动的提示词输入区
自定义 status line（配置后）
内建 footer：模式、代理、PR、告警等
~~~

**位置：**从终端顶部到最底部；自定义 status line 明确位于输入区之下、内建 footer badges 之上。  
**证据：**【文档事实；示意图为综合归纳】[Interactive mode](https://code.claude.com/docs/en/interactive-mode)、[Status line](https://code.claude.com/docs/en/statusline)、[Fullscreen mode](https://code.claude.com/docs/en/fullscreen)（访问：2026-08-04）

### 1.3 启动标识与会话身份

**名称：Claude 方块字标与启动摘要。**  
**外观：**官方仓库中 Claude Code v2.1.114 的 Windows 转录呈现三行字符字标，右侧依次是版本、模型/effort/订阅和当前目录：

~~~text
 ▐▛███▜▌   Claude Code v2.1.114
▝▜█████▛▘  Opus 4.7 with high effort · Claude Max
  ▘▘ ▝▝    C:\Users\TJ
~~~

**位置：**新会话 Transcript 顶部。不同版本、登录方式和终端宽度可增减右侧字段。  
**证据：**【官方仓库观察，v2.1.114；不是当前版本固定模板】[anthropics/claude-code#50803](https://github.com/anthropics/claude-code/issues/50803)（访问：2026-08-04）

**名称：主题与语义色层。**  
**外观：**内建主题包括 automatic、light、dark、daltonized 与 ANSI 方案；终端本身控制底色。官方可配置语义色包括主强调 <code>claude</code>、正文 <code>text</code>、弱化 <code>inactive</code>/<code>subtle</code>、建议 <code>suggestion</code>、权限 <code>permission</code>、记忆 <code>remember</code>，以及 success/error/warning/merged 等状态色。  
**位置：**全局作用于标识、消息、边框、状态、diff 与选择态。  
**证据：**【文档事实；具体 RGB 不确定】[Terminal configuration：themes](https://code.claude.com/docs/en/terminal-config)（访问：2026-08-04）

## 2 Transcript

### 2.1 消息角色与层级符号

**名称：用户提示符。**  
**外观：**官方仓库多份转录以 <code>❯</code> 开始用户输入，例如 <code>❯ /context</code>；旧版本或纯文本复制中也可能退化为 <code>&gt;</code>/<code>&gt;&gt;</code>。fullscreen 还可为整条用户消息铺设语义背景色。  
**位置：**每条用户消息开头；正在编辑的提示词也位于底部输入区。  
**证据：**【官方仓库观察 + 文档事实】[anthropics/claude-code#50803](https://github.com/anthropics/claude-code/issues/50803)、[Fullscreen themes](https://code.claude.com/docs/en/fullscreen)（访问：2026-08-04）

**名称：Claude / 工具活动圆点。**  
**外观：**常见主层级前缀是实心样式 <code>⏺</code>，例如 <code>⏺ Read(.mcp.json)</code> 或 <code>⏺ Bash(python3 ...)</code>。具体颜色由主题的 Claude 主强调色决定。  
**位置：**Transcript 左侧主层级，位于 Claude 内容或工具调用行前。  
**证据：**【官方仓库观察；字符在不同字体中宽度可能不同】[anthropics/claude-code#8139](https://github.com/anthropics/claude-code/issues/8139)、[anthropics/claude-code#37394](https://github.com/anthropics/claude-code/issues/37394)（访问：2026-08-04）

**名称：思考 spinner。**  
**外观：**仓库转录中的静态帧为 <code>✻ Thinking…</code>；运行时该符号/亮度会动画，动词可能变化，因此 <code>Thinking…</code> 不能视为唯一文案。  
**位置：**用户消息之后、Claude 正在推理但尚未给出下一块可见内容时。  
**证据：**【官方仓库观察；动词与帧不稳定】[anthropics/claude-code#2988](https://github.com/anthropics/claude-code/issues/2988)、[Terminal configuration：spinner theme tokens](https://code.claude.com/docs/en/terminal-config)（访问：2026-08-04）

**名称：工具结果子层级。**  
**外观：**工具调用下方缩进，以 <code>⎿</code> 连接结果。例如：

~~~text
⏺ Read(.mcp.json)
  ⎿ Read 34 lines (ctrl+o to expand)
~~~

**位置：**紧随对应工具调用，形成“主圆点 → 缩进结果”的树状层级。  
**证据：**【官方仓库观察】[anthropics/claude-code#8139](https://github.com/anthropics/claude-code/issues/8139)（访问：2026-08-04）

### 2.2 工具调用、折叠与摘要

**名称：工具调用标题行。**  
**外观：**通常采用 <code>工具名(主要参数)</code>，例如 <code>Read(.mcp.json)</code>、<code>Bash(python3 ...)</code>；工具名与参数形成单行标题，长参数会折叠或截断。fullscreen 中整个调用及结果可点击展开/收起。  
**位置：**Transcript 的 Claude 活动层级中。  
**证据：**【文档事实 + 官方仓库观察】[Fullscreen mouse support](https://code.claude.com/docs/en/fullscreen)、[anthropics/claude-code#37394](https://github.com/anthropics/claude-code/issues/37394)（访问：2026-08-04）

**名称：多行结果折叠尾。**  
**外观：**精确观察格式为 <code>… +6 lines (ctrl+o to expand)</code>，即省略号、加号、隐藏行数与快捷键提示；Read 的摘要可写成 <code>Read 34 lines (ctrl+o to expand)</code>。当前 changelog 仍提到此展开 affordance，但不同工具的摘要措辞不同。  
**位置：**折叠工具结果的最后一行，缩进在 <code>⎿</code> 之下。  
**证据：**【官方仓库观察 + 当前官方变更记录】[anthropics/claude-code#37394](https://github.com/anthropics/claude-code/issues/37394)、[Claude Code CHANGELOG](https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md)（访问：2026-08-04）

**名称：MCP 调用聚合摘要。**  
**外观：**重复 MCP 调用可折成一行，例如官方文档给出的 <code>Called slack 3 times</code>；展开后才显示各次调用细节。  
**位置：**普通 Transcript 中替代一组连续 MCP 调用。  
**证据：**【文档事实】[Interactive mode：transcript mode](https://code.claude.com/docs/en/interactive-mode)（访问：2026-08-04）

**名称：子代理任务树摘要。**  
**外观：**官方仓库 issue 中一个已完成批次呈现为标题、树枝、工具次数、token 数和 <code>Done</code>：

~~~text
6 Task agents finished (ctrl+o to expand)
├─ Search Global Job Portals · 15 tool uses · 23.0k tokens
│ ⎿  Done
...
└─ Search Big 4 Consultancies · 16 tool uses · 18.3k tokens
  ⎿  Done
~~~

**位置：**主 Transcript 中，作为一组 Task/subagent 工作的折叠结果。  
**证据：**【官方仓库观察，特定版本；任务数、缩进和字段可能变化】[anthropics/claude-code#16157](https://github.com/anthropics/claude-code/issues/16157?timeline_page=1)（访问：2026-08-04）

**名称：delegation 单行。**  
**外观：**官方 subagent 文档示例为 <code>code-improver (Suggest code improvements)</code>，以代理名加括号内描述标识委派。  
**位置：**主 Transcript 的委派调用处。  
**证据：**【文档事实】[Create custom subagents](https://code.claude.com/docs/en/sub-agents)（访问：2026-08-04）

### 2.3 Transcript 的显示模式

**名称：详细 Transcript viewer。**  
**外观：**<code>Ctrl+O</code> 切换详细转录；显示原本折叠的工具调用/执行细节，并在每条 assistant message 上显示时间戳与模型。fullscreen 中它是可导航的专用视图，可用 <code>?</code> 看按键。  
**位置：**覆盖/替代主 Transcript 区域；退出后回到普通会话。  
**证据：**【文档事实】[Interactive mode：transcript mode](https://code.claude.com/docs/en/interactive-mode)、[Fullscreen transcript mode](https://code.claude.com/docs/en/fullscreen)（访问：2026-08-04）

**名称：Focus view。**  
**外观：**<code>/focus</code> 只保留最近一次用户 prompt、每个工具调用的一行摘要（编辑类调用带 diffstat）和最终回答，压低此前上下文的视觉噪声。  
**位置：**fullscreen 的 Transcript 主区域。  
**证据：**【文档事实】[Fullscreen mode：focus view](https://code.claude.com/docs/en/fullscreen)（访问：2026-08-04）

**名称：shell mode 记录。**  
**外观：**输入以 <code>!</code> 为前缀；命令输出实时流入 Transcript。fullscreen 主题还为 bash entries 提供独立背景 token，因此 shell 记录可与对话消息视觉分层。  
**位置：**底部 prompt 发起，结果进入上方 Transcript。  
**证据：**【文档事实】[Interactive mode：shell mode](https://code.claude.com/docs/en/interactive-mode)、[Terminal configuration：fullscreen tokens](https://code.claude.com/docs/en/terminal-config)（访问：2026-08-04）

**名称：会话恢复摘要。**  
**外观：**长时间离开后回到会话，会出现一行 session recap，概括离开期间发生的事；官方未把字符前缀或颜色规定为稳定格式。  
**位置：**恢复焦点后的 Transcript/状态邻近区域。  
**证据：**【文档事实；精确字形不确定】[Interactive mode：session recap](https://code.claude.com/docs/en/interactive-mode)（访问：2026-08-04）

### 2.4 Transcript 内的输入附件

**名称：图片占位 chip。**  
**外观：**粘贴图片后在光标处插入精确形式 <code>[Image #N]</code>；N 是当前输入中的序号，chip 代表二进制图片而非把图片像素直接铺进终端。  
**位置：**底部 prompt 编辑器的文本流内，提交后随用户消息进入 Transcript。  
**证据：**【文档事实】[Interactive mode：paste images](https://code.claude.com/docs/en/interactive-mode)（访问：2026-08-04）

**名称：大段粘贴占位。**  
**外观：**普通 prompt 对超过 10,000 字符的粘贴显示 <code>[Pasted text]</code>；Agent View dispatch 对超过 800 字符或 2 行的内容显示带编号的 <code>[Pasted text #N]</code>。  
**位置：**各自底部输入框内，代替长文本占满视口。  
**证据：**【文档事实】[Interactive mode：pasting text](https://code.claude.com/docs/en/interactive-mode)、[Agent View：dispatch input](https://code.claude.com/docs/en/agent-view)（访问：2026-08-04）

## 3 底部状态行

### 3.1 Prompt 输入区

**名称：主 prompt 编辑器。**  
**外观：**可多行编辑的输入框；官方主题为默认、plan、auto-accept、bash、IDE、fast mode 分别定义边框语义色。fullscreen 中它固定在屏幕底部，classic 中随输出追加向下移动。仓库旧版截图可见圆角框线 <code>╭─╮ / │ &gt; │ / ╰─╯</code>，但边框字符不是当前文档保证。  
**位置：**Transcript 下方、status line 与 footer 上方。  
**证据：**【文档事实 + 官方仓库观察】[Fullscreen mode](https://code.claude.com/docs/en/fullscreen)、[Terminal configuration：prompt border tokens](https://code.claude.com/docs/en/terminal-config)、[anthropics/claude-code#7261](https://github.com/anthropics/claude-code/issues/7261)（访问：2026-08-04）

**名称：输入语法触发符。**  
**外观：**空 prompt 输入 <code>/</code> 打开 slash command 菜单，<code>!</code> 进入 shell mode，<code>@</code> 打开文件路径补全，<code>:</code> 打开 emoji 补全。符号既是输入内容也是视觉模式提示。  
**位置：**prompt 光标处及其上方/邻近的补全列表。  
**证据：**【文档事实】[Interactive mode：quick commands](https://code.claude.com/docs/en/interactive-mode)（访问：2026-08-04）

**名称：灰色 prompt suggestion。**  
**外观：**建议以灰色、未提交文本出现在 prompt 内；<code>Tab</code> 或右方向键接受，继续输入会消失。其语义色 token 名为 <code>suggestion</code>。  
**位置：**当前 prompt 文本之后，和光标同一编辑区。  
**证据：**【文档事实】[Interactive mode：prompt suggestions](https://code.claude.com/docs/en/interactive-mode)、[Terminal configuration](https://code.claude.com/docs/en/terminal-config)（访问：2026-08-04）

### 3.2 自定义 status line 与内建 footer

**名称：自定义 status line。**  
**外观：**脚本 stdout 每一行占一行，可输出 ANSI 颜色和 OSC 8 超链接。官方单行示例为 <code>[$MODEL] 📁 ${DIR##*/} | ${PCT}% context</code>；进度表示例使用 10 格 <code>▓</code>/<code>░</code>。可配置 padding，也可多行。  
**位置：**独占 prompt 下方、内建 footer badges 上方的一行或多行；不会替换内建 footer。自动补全、帮助和权限提示期间可暂时隐藏。  
**证据：**【文档事实】[Status line](https://code.claude.com/docs/en/statusline)（访问：2026-08-04）

**名称：权限模式徽标。**  
**外观：**当前文档给出的精确默认编辑接受提示是 <code>⏵⏵ accept edits on</code>。官方仓库旧版还观察到 <code>⏸ plan mode on (shift+tab to cycle)</code> 与 <code>⏵⏵ bypass permissions on (shift+tab to cycle)</code>；<code>Shift+Tab</code> 循环模式。模式同时映射到不同 prompt 边框语义色。  
**位置：**prompt 下方的内建 footer 左侧。  
**证据：**【文档事实；长文案为官方仓库版本观察】[Permission modes](https://code.claude.com/docs/en/permission-modes)、[anthropics/claude-code#45453](https://github.com/anthropics/claude-code/issues/45453)、[Terminal configuration](https://code.claude.com/docs/en/terminal-config)（访问：2026-08-04）

**名称：Vim 模式指示器。**  
**外观：**Vim 编辑启用时，prompt 下方出现精确文字 <code>-- INSERT --</code>；可通过 <code>hideVimModeIndicator</code> 隐藏。  
**位置：**prompt 正下方的 footer 区。  
**证据：**【文档事实】[Interactive mode：Vim editor](https://code.claude.com/docs/en/interactive-mode)（访问：2026-08-04）

**名称：任务 checklist 状态区。**  
**外观：**<code>Ctrl+T</code> 展开/收起 Claude 的 to-do checklist，最多同时显示 5 个任务；这是持续状态区，不等同于 <code>/tasks</code> 后台任务管理器。具体 checkbox 字形未在文档中固定。  
**位置：**prompt 上方或紧邻其上的 status area。  
**证据：**【文档事实；checkbox 精确字符不确定】[Interactive mode：task list](https://code.claude.com/docs/en/interactive-mode)（访问：2026-08-04）

**名称：子代理 footer hint。**  
**外观：**存在代理时显示 <code>← for agents</code>；多代理时可为 <code>← 2 agents</code>，计数最多显示 <code>99+</code>。完成时会短暂闪成 <code>← 2 done</code>，随后恢复；约每 10 秒刷新。  
**位置：**内建 footer，提示按左方向键进入代理界面。  
**证据：**【文档事实】[Subagents：agent status in the footer](https://code.claude.com/docs/en/sub-agents)（访问：2026-08-04）

**名称：Pull Request 徽标。**  
**外观：**例如 <code>PR #446</code> 的可点击/可打开链接；下划线颜色表达 review 状态：绿色 approved、黄色 pending、红色 changes requested、灰色 draft。合并或关闭后消失。  
**位置：**内建 footer。  
**证据：**【文档事实】[Interactive mode：pull request status](https://code.claude.com/docs/en/interactive-mode)（访问：2026-08-04）

**名称：模型 effort 控件。**  
**外观：**官方仓库一个版本的右侧 footer 观察为 <code>● medium · /effort</code>，与左侧权限文案分列；圆点、文字和命令提示组成紧凑控件。当前精确布局会随宽度和版本变化。  
**位置：**内建 footer 右侧。  
**证据：**【官方仓库观察，非稳定模板】[anthropics/claude-code#45453](https://github.com/anthropics/claude-code/issues/45453)（访问：2026-08-04）

**名称：通知与 token/上下文状态。**  
**外观：**MCP errors、auto-update、context-low warning 等通知与自定义 status line 共用底部行；verbose 模式可增加 token counter。窄终端会截断，多个瞬态通知会轮换。  
**位置：**底部 status/footer 右侧或共享空位。  
**证据：**【文档事实；排列取决于终端宽度】[Status line：built-in notifications](https://code.claude.com/docs/en/statusline)（访问：2026-08-04）

**名称：语音输入提示与波形。**  
**外观：**空 prompt 可显示 <code>hold Space to speak</code>；按住后的预热提示为 <code>keep holding…</code>，开始录音后显示实时 waveform，尚未最终确认的转写文本以弱化样式显示。配置自定义 status line 后，空 prompt 的语音提示会被抑制。  
**位置：**prompt/footer 邻近区域；波形位于正在输入的语音状态中。  
**证据：**【文档事实】[Voice dictation](https://code.claude.com/docs/en/voice-dictation)、[Status line](https://code.claude.com/docs/en/statusline)（访问：2026-08-04）

## 4 浮层与对话框

### 4.1 权限与计划确认

**名称：工具权限确认框。**  
**外观：**带标题、目标摘要、提问和编号选项；当前选择以前导 <code>❯</code> 标识。官方仓库 v2.0.36 的 Read 权限转录为：

~~~text
Read file

  Read(AP-Cleanup-Documentation.md)

Do you want to proceed?
❯ 1. Yes
  2. Yes, during this session
  3. No, and tell Claude what to do differently (esc)
~~~

**位置：**主 Transcript 下部、prompt 上方/原 prompt 所在交互焦点；弹出时自定义 status line 可隐藏。fullscreen 可用鼠标悬停并选择。  
**证据：**【官方仓库观察，v2.0.36 文案；结构由当前文档确认】[anthropics/claude-code#11380](https://github.com/anthropics/claude-code/issues/11380?timeline_page=1)、[Fullscreen mouse support](https://code.claude.com/docs/en/fullscreen)、[Status line](https://code.claude.com/docs/en/statusline)（访问：2026-08-04）

**名称：计划批准对话框。**  
**外观：**计划完成后显示可选行动。当前文档列出的标签包括 <code>Yes, and use auto mode</code>（某些配置为 <code>Yes, auto-accept edits</code>）、<code>Yes, manually approve edits</code>、<code>No, refine with Ultraplan on Claude Code on the web</code>、<code>No, keep planning</code>；启用 bypass 时第一项可为 <code>Yes, and bypass permissions</code>。  
**位置：**plan mode 的 Transcript 底部，接替普通 prompt 成为当前交互焦点。  
**证据：**【文档事实；选项会依设置变化】[Permission modes：plan approval](https://code.claude.com/docs/en/permission-modes)（访问：2026-08-04）

**名称：后台子代理权限提示。**  
**外观：**与普通权限框同类，但提示会注明是哪一个 subagent 请求权限；即使该代理在后台运行，确认仍浮到主会话。  
**位置：**主会话当前 prompt 附近，而不是藏在不可见的子代理 Transcript 中。  
**证据：**【文档事实；精确标题随工具变化】[Subagents：permission prompts](https://code.claude.com/docs/en/sub-agents)（访问：2026-08-04）

### 4.2 补全、帮助与临时问答

**名称：slash command 补全菜单。**  
**外观：**空 prompt 输入 <code>/</code> 后显示命令名及说明的可筛选列表；可用方向键移动并确认。命令本身采用 <code>/command</code> 形态。  
**位置：**prompt 上方或贴近 prompt 的浮层列表。  
**证据：**【文档事实；边框/行色不作固定承诺】[Interactive mode](https://code.claude.com/docs/en/interactive-mode)、[Commands](https://code.claude.com/docs/en/commands)（访问：2026-08-04）

**名称：路径、emoji 与 subagent typeahead。**  
**外观：**<code>@</code> 后出现文件路径候选，<code>:</code> 后出现 emoji 候选；输入 <code>@&lt;name&gt;</code> 可出现代理名及状态的 typeahead。选中行使用主题选择态。  
**位置：**当前 prompt 邻近的候选浮层。  
**证据：**【文档事实；具体高亮色由主题决定】[Interactive mode](https://code.claude.com/docs/en/interactive-mode)、[Subagents：mention subagents](https://code.claude.com/docs/en/sub-agents)（访问：2026-08-04）

**名称：快捷键帮助面板。**  
**外观：**空 prompt 按 <code>?</code> 展开/收起快捷键面板；专用 Transcript/Agent View 中也可用 <code>?</code> 显示该视图自己的按键帮助。  
**位置：**主界面或当前全屏视图上的临时面板。  
**证据：**【文档事实；具体行文随当前视图变化】[Interactive mode：keyboard shortcuts](https://code.claude.com/docs/en/interactive-mode)、[Fullscreen mode](https://code.claude.com/docs/en/fullscreen)、[Agent View](https://code.claude.com/docs/en/agent-view)（访问：2026-08-04）

**名称：<code>/btw</code> 临时问答浮层。**  
**外观：**当前问题和答案成为主焦点，较早的临时问答以弱化列表留在其上；长答案可滚动。左右方向键切换，<code>c</code> 复制，<code>f</code> fork 为新会话，<code>x</code> 清空。关闭后不会把该问答写入主对话上下文。  
**位置：**覆盖主 Transcript 的临时层，底层会话保持不变。  
**证据：**【文档事实】[Interactive mode：side questions with /btw](https://code.claude.com/docs/en/interactive-mode)（访问：2026-08-04）

### 4.3 搜索、设置与反馈浮层

**名称：反向历史搜索对话框。**  
**外观：**fullscreen 中 <code>Ctrl+R</code> 打开专用搜索框，匹配查询文字会高亮；上下键在结果间移动，<code>Ctrl+S</code> 切换搜索范围。classic 则使用行内历史搜索，不是相同浮层。  
**位置：**fullscreen 主界面的居中/叠加对话层，关闭后返回 prompt。  
**证据：**【文档事实】[Fullscreen mode：reverse history search](https://code.claude.com/docs/en/fullscreen)（访问：2026-08-04）

**名称：带 tabs 的设置类对话框。**  
**外观：**<code>/config</code>、<code>/permissions</code> 等界面可有横向 tabs；左右方向键切换，<code>Esc</code> 关闭。<code>/permissions</code> 按 scope 显示规则、工作目录，并有 Recent denials tab。  
**位置：**覆盖主会话的 modal 区域。  
**证据：**【文档事实；具体 tab 名受版本/配置影响】[Interactive mode：dialog navigation](https://code.claude.com/docs/en/interactive-mode)、[Permission modes](https://code.claude.com/docs/en/permission-modes)、[Commands](https://code.claude.com/docs/en/commands)（访问：2026-08-04）

**名称：模型选择器。**  
**外观：**<code>/model</code> 打开模型列表/选择器；左右方向键调整 effort，<code>s</code> 可将选择限定为当前 session。选择项、当前项和 effort 构成同一对话框。  
**位置：**主会话上的 modal。  
**证据：**【文档事实】[Commands：/model](https://code.claude.com/docs/en/commands)（访问：2026-08-04）

**名称：主题选择器。**  
**外观：**<code>/theme</code> 打开主题列表并即时预览；<code>Ctrl+T</code> 切换代码 syntax highlighting。选项包括内建主题及已安装自定义主题。  
**位置：**主会话上的 modal，预览效果作用于界面。  
**证据：**【文档事实】[Terminal configuration：theme picker](https://code.claude.com/docs/en/terminal-config)（访问：2026-08-04）

**名称：滚动速度调节框。**  
**外观：**<code>/scroll-speed</code> 显示一条 ruler；左右方向键调节，<code>r</code> 重置，<code>Enter</code> 保存。  
**位置：**fullscreen 上的设置 modal。  
**证据：**【文档事实】[Fullscreen mode：scroll speed](https://code.claude.com/docs/en/fullscreen)（访问：2026-08-04）

**名称：复制完成 toast。**  
**外观：**复制 transcript/消息后短暂出现提示，并说明所采用的复制路径；官方未规定固定边框字符。  
**位置：**fullscreen 中临时叠加于当前视图，不占永久布局。  
**证据：**【文档事实；精确措辞不确定】[Fullscreen mode：copying text](https://code.claude.com/docs/en/fullscreen)（访问：2026-08-04）

**名称：强制可见的响应对话框。**  
**外观：**即使用户已经向上滚动并暂停 auto-follow，需要用户回答的 modal 仍会滚入可视区，避免权限或选择请求藏在下方。  
**位置：**fullscreen 当前可视区域内。  
**证据：**【文档事实】[Fullscreen mode：auto-follow and dialogs](https://code.claude.com/docs/en/fullscreen)（访问：2026-08-04）

## 5 面板与独立视图

### 5.1 Tasks、subagents 与 forks

**名称：<code>/tasks</code> 任务管理视图。**  
**外观：**列出后台任务及状态，可检查、管理运行项；它是任务执行管理界面，不是 <code>Ctrl+T</code> 的五项 to-do checklist。  
**位置：**从主会话打开的独立任务视图。  
**证据：**【文档事实；官方未固定每列字符宽度】[Interactive mode：background tasks](https://code.claude.com/docs/en/interactive-mode)、[Commands：/tasks](https://code.claude.com/docs/en/commands)（访问：2026-08-04）

**名称：subagent 状态面板。**  
**外观：**默认每行由 <code>name · description · token count</code> 组成；可用 <code>subagentStatusLine</code> 自定义。每个 subagent 被分配 red、blue、green、yellow、purple、orange、pink、cyan 八种具名色之一，以便在并发输出中保持身份辨识。  
**位置：**prompt 下方/附近的 agent panel；代理运行时持续可见。  
**证据：**【文档事实；实际 RGB 由主题定义】[Status line：subagent status line](https://code.claude.com/docs/en/statusline)、[Subagents：subagent colors](https://code.claude.com/docs/en/sub-agents)（访问：2026-08-04）

**名称：Running forks 面板。**  
**外观：**主会话和每个 fork 各占一行，显示运行状态；<code>Enter</code> 打开某个 subagent transcript，<code>x</code> dismiss/stop，<code>Esc</code> 回到 prompt。  
**位置：**prompt 下方的临时运行面板。  
**证据：**【文档事实；精确状态图标未固定】[Subagents：running forks](https://code.claude.com/docs/en/sub-agents)（访问：2026-08-04）

### 5.2 Agent View

**名称：Agent View 总表。**  
**外观：**全屏、多分组的 session table；顶部 header 显示 Claude Code 版本、模型、cwd 和摘要计数，底部是 dispatch input 与按键提示。官方文档给出的核心列表示例如下：

~~~text
Pinned
  ✽ clawd walk cycle          Drawing the walk-cycle sprite frames          3m

Ready for review
  ∙ jump physics              Opened PR with collision fix                 #2048  2h

Needs input
  ✻ power-up design           double jump or wall climb?                    1m

Working
  ✽ collision detection       Adding swept-AABB checks to CollisionSystem   2m
  ✢ playtest level 3          run 12 · all checkpoints cleared           in 4m

Completed
  ✻ title screen              result: menu, options, and credits done       9m
  ∙ sound effects             result: 14 SFX exported to assets/audio       4h
  … 6 more
~~~

**位置：**由 <code>claude agents</code> 或相关入口打开的独立 terminal TUI。  
**证据：**【文档事实；Agent View 标为 research preview，界面可快速变化】[Agent View](https://code.claude.com/docs/en/agent-view)（访问：2026-08-04）

**名称：Agent View 状态图标。**  
**外观：**<code>✻</code>/<code>✽</code> 表示活跃并可动画，<code>∙</code> 表示进程已退出，<code>✢</code> 表示 loop 正在 sleeping。Working 会动画；Needs input 为黄色；Idle 弱化；Completed 绿色；Failed 红色；Stopped 灰色。  
**位置：**每个 session row 最左侧。  
**证据：**【文档事实；色名是语义，实际色值随主题】[Agent View：status icons and colors](https://code.claude.com/docs/en/agent-view)（访问：2026-08-04）

**名称：Agent View session row。**  
**外观：**从左到右是状态图标、session 名、当前 activity、可选 PR <code>#N</code>、最右侧 age；session 名可用 <code>/color</code> 选择的色调显示。Completed 项过多时折为 <code>… N more</code>。  
**位置：**Pinned、Ready for review、Needs input、Working、Completed 等分组之下。  
**证据：**【文档事实】[Agent View：session list](https://code.claude.com/docs/en/agent-view)（访问：2026-08-04）

**名称：Agent peek panel。**  
**外观：**按 <code>Space</code> 打开，展示完整问题/结果/状态句、关联 PR、等待时长（如 <code>waiting 3m</code>）及 reply input。普通问题可显示编号选项；权限请求显示权限文本而不是编号选项。  
**位置：**Agent View 当前 session 上的详情面板。  
**证据：**【文档事实】[Agent View：peek and respond](https://code.claude.com/docs/en/agent-view)（访问：2026-08-04）

**名称：Agent View dispatch input。**  
**外观：**固定在表格底部，用来给新/现有 session 派发任务；长粘贴折为 <code>[Pasted text #N]</code>，其下 footer 显示当前 dispatch defaults。  
**位置：**Agent View 最底部，键盘提示之上或相邻。  
**证据：**【文档事实】[Agent View：dispatch input](https://code.claude.com/docs/en/agent-view)（访问：2026-08-04）

**名称：Agent View terminal tab title。**  
**外观：**普通状态为 <code>claude agents</code>；有两项等待输入时可变为 <code>2 awaiting input · claude agents</code>。  
**位置：**终端模拟器标签页/窗口标题，不在字符画布内部。  
**证据：**【文档事实】[Agent View：terminal title](https://code.claude.com/docs/en/agent-view)（访问：2026-08-04）

**名称：后台启动回执块。**  
**外观：**从 shell 后台启动 agent 后打印 session id、名称和四条后续命令：

~~~text
backgrounded · 7c5dcf5d · flaky-test-fix
  claude agents             list sessions
  claude attach 7c5dcf5d    open in this terminal
  claude logs 7c5dcf5d      show recent output
  claude stop 7c5dcf5d      stop this session
~~~

**位置：**启动命令所在 shell 的普通输出区；它是 Agent View 的入口回执，不是 view 内部的 panel。  
**证据：**【文档事实】[Agent View：start agents in background](https://code.claude.com/docs/en/agent-view)（访问：2026-08-04）

### 5.3 上下文、diff 与管理视图

**名称：<code>/context</code> 上下文可视化。**  
**外观：**当前文档定义为 colored grid，加按类别的 token breakdown 与优化建议；fullscreen 默认折叠 item breakdown 以保留 grid，可用 <code>/context all</code> 展开。官方仓库 v2.1.114 的纯文本观察曾显示：

~~~text
Context Usage
Opus 4.7
claude-opus-4-7
22.4k/200k tokens (11%)

System prompt:    8.3k tokens (4.1%)
System tools:    13.1k tokens (6.6%)
Memory files:     298 tokens (0.1%)
Skills:           689 tokens (0.3%)
Messages:          13 tokens (0.0%)
Free space:     144.6k (72.3%)
Autocompact buffer: 33k tokens (16.5%)
~~~

**位置：**从主会话打开的独立上下文面板。  
**证据：**【当前文档事实 + 官方仓库 v2.1.114 观察；旧转录不代表当前 grid 字符】[Commands：/context](https://code.claude.com/docs/en/commands)、[Context windows](https://code.claude.com/docs/en/context-window)、[anthropics/claude-code#50803](https://github.com/anthropics/claude-code/issues/50803)（访问：2026-08-04）

**名称：<code>/diff</code> 独立 diff viewer。**  
**外观：**左右切换当前 git diff 与各 turn，文件列表可上下移动，<code>Enter</code> 打开文件 diff，内容可滚动，<code>Esc</code> 返回。增删行、上下文及词内变化使用不同语义色。  
**位置：**覆盖主 Transcript 的独立视图。  
**证据：**【文档事实】[Interactive mode：diff viewer](https://code.claude.com/docs/en/interactive-mode)、[Terminal configuration：diff tokens](https://code.claude.com/docs/en/terminal-config)（访问：2026-08-04）

**名称：Transcript 独立查看器。**  
**外观：**fullscreen 下 <code>Ctrl+O</code> 进入可搜索、可按 message/tool block 导航的全屏 Transcript；<code>[</code> 把完整展开会话写入原生 scrollback，<code>v</code> 用临时编辑器打开。  
**位置：**占据主内容区，退出后回到会话。  
**证据：**【文档事实】[Fullscreen mode：transcript mode](https://code.claude.com/docs/en/fullscreen)（访问：2026-08-04）

**名称：设置与诊断类独立视图。**  
**外观：**<code>/status</code> 的 Status tab 汇总版本、模型、账户与连接；<code>/stats</code> 打开使用统计；<code>/resume</code> 显示可恢复会话选择器；<code>/usage</code> 显示计划用量/限制。它们共享终端列表、分组、当前选择高亮等视觉语法，但官方未承诺统一的列宽或框线字符。  
**位置：**由 slash command 从主会话打开的 panel/view。  
**证据：**【文档事实；逐视图像素细节不确定】[Commands](https://code.claude.com/docs/en/commands)（访问：2026-08-04）
