
## 2026-07-03 · SMMP 推广渠道决策

- **结论**:闲鱼/拼多多/淘宝 不作为主力,只做闲鱼 1-2 个号测试
- **国内打法**:知乎 + B 站 + 小红书 内容种草 → 主页引到 TG 私域成交,**全程 USDT 收款**
- **国外主力**:TG 群 60% + Discord 15% + X 10% + Reddit 10% + 跨境论坛 5%
- **预算节奏**:M1 0 元 → M2-M3 1000-5000 元/月 → M4+ 5000-10000 元/月
- **里程碑**:M2 末 100-200 客户 / 月流水 1-3 万;M6 末月流水 15-30 万
- **关联**:[[03-推广方案与渠道分析]] / [[TG群运营SOP]]

## 2026-07-03 · SMMP 完整营销规划定稿

- **优先级框架**:P0 TG 群(50%)→ P1 国外公域(25%)→ P2 国内种草(15%)→ P3 冷邮件(5%)→ P4 长尾(5%)
- **6 个月里程碑**:M1 50 种子 → M2 150 客户/2 万 → M3 400 客户/8 万 → M6 1000 客户/25 万
- **总预算**:3-5 万 RMB 纯人工 / 6-10 万含外包(6 个月)
- **客户分层**:C 端 80% / B 端小工作室 15% / 代理 5%
- **冷邮件**:M3 开始,主攻 MCN/工作室 B 端,客单价 200-1000 USD
- **关键节点**:M2 末必须达到月流水 2 万(自给自足),否则重审策略
- **关联**:[[04-完整营销规划]] / [[03-推广方案与渠道分析]] / [[TG群运营SOP]]

## 2026-07-03 v0.8.18 部署

**决定**:
1. 定价公式 = 上游 × 2(不再 × 2 × 6.70)
2. DB 存 USD cents(整数美分)而非 CNY cents
3. 服务数 873 → 5140(BulkFollows 全量)
4. Bundle 85 折保持
5. D1 IN 子句分块 100 解决 /services/instagram 500

**影响**:
- 英文用户看到美元价格更直观
- 中文字面 ¥ 显示 = USD × 6.70 汇率
- 14 倍服务数让用户能挑到具体国家/quality/服务类型

**风险**:
- 服务名 regex 解析 edge case 多了,需要后续看监控告警
- D1 size 涨到 4.2MB(5180 行 × 4 表),还在免费层范围内

## 2026-07-03 v0.8.29 部署 + 运营 SOP 重写

**决定**:
1. 部署 v0.8.29(7 项封板小修: topup zh/en 货币区分 / 注册送 ¥5/$5 / 首页 ¥15 起步 / platform SEO 同步 / i18n 5 处修复 / translateRefillLabel 重写 / 2 个 typecheck 修)
2. 重写运营 SOP 对齐当前真实数据:
   - 13 平台 × 823 active 服务 / 6 bundle(85 折)
   - 起步价统一 ≥ ¥15/1000(Spotify/Web 例外保留)
   - 客户分 3 档(C 端 70% / B 端 20% / 代理 10%)
   - 渠道权重:TG 50% / 知乎 15% / 小红书 10% / B 站 10% / Discord+X 10% / 公众号 5%
3. 30 天落地计划: M1 目标 50 付费 / $1500-2000 流水

**影响**:
- 客户首单转化率预估 30%(注册送 ¥5 体验金)
- 客单价 C 端 $20-30 / B 端 $200-500 / 代理 $1000+
- 单客毛利 50-65%($15-20 / $30 月充)
- 客服 / 内容 外包 2 个, M1 月成本 ¥5000-9000

**风险**:
- TG 群被封概率中(50%精力在 TG,需备份群)
- 知乎 / 小红书账号被封(备用号 + Notion 备份)
- 退款率 > 10% 立即关停问题服务

**关联**:
- [[13-v0.8.29-运营SOP-TG+国内]] 完整 SOP
- [[TG群运营SOP]] TG 私域打法(已有)
- [[03-推广方案与渠道分析]] 渠道分析
- [[04-完整营销规划]] 6 个月里程碑
- [[决策日志]] 2026-07-03 v0.8.18 / 2026-07-03 v0.8.29

## 2026-07-03 v0.8.29 SOP: 增加 QQ 群潜群引流打法(§5.5)

**决定**:
1. 在运营 SOP 增加 §5.5 QQ 群潜群引流(国内第 4 渠道,5% 精力)
2. 定位: **不建群,只潜入别人跨境 QQ 群**(独立站/TikTok/Affiliate/IG 圈)
3. 核心打法: 14 天养号 + 专业回答建立人设 + 签名图引流 TG + 1v1 私聊转化
4. 渠道权重调整: 公众号 3%→2%, 新增 QQ 群潜群 5%

**关键动作**:
- 准备 3 个 QQ 小号(大号/业务号/备用号), 不同手机/IP/型号
- 7 天养号(普通群→兴趣群→跨境群)
- 14 天进群脚本: D1-D3 潜水 → D4-D7 专业回答 → D8-D10 案例分享 → D11-D14 私聊转化
- 签名图模板: TG 二维码 + 备注"QQ"送 13 平台价格表
- 1v1 私聊 7 步法: 钩子→探需→方案→报价→替代→TG 导流→收尾

**防踢 8 条红线**:
- ❌ 不在群内发 USDT 收款码 / TG 群链接 / 具体价格(¥xx 包)
- ❌ 不用同一句话发 3 次 / 不 @ 全体成员
- ❌ 不加群主/管理员好友
- ❌ 不 1 天加 5 个以上跨境群
- ❌ 涉政/赌博/黄/账号密码(永久封号)

**ROI 预期**:
- 单 500 人 QQ 群 = 每天 1-3 条私聊, 月转化 2-5 客户
- 私聊转化率 5-10%(vs TG 10-15%)
- 5% 精力换 2-5 客户/月(占比 5%)
- 关键: 业务号 + 养号 + 私聊, **绝不靠"群内发广告"**

**关联**: [[13-v0.8.29-运营SOP-TG+国内]] §5.5

## 2026-07-03 v0.8.29 SOP: 30 天计划补 QQ 群潜群任务

**决定**:
1. 在 §10 30 天落地计划里增加 QQ 群潜群任务(贯穿 W1-W4)
2. W1 重点: **D1 准备 3 个小号 + D2-D7 业务号养号 7 天 + 累计加 8-10 个跨境群**(只潜水不发言)
3. W2 重点: 业务号开始"专业回答"(D8-D10) + 主动私聊(D11-D14) + 互推群加 3 个
4. W3 重点: 业务号覆盖 10-13 群, 每天 5-8 条私聊, 7 步成交法, 送 ¥5 体验金
5. W4 重点: 业务号 + 备用号双号并行, 覆盖 15-20 群, 客户交叉到 Notion CRM(标 source=qq)

**M1 末 QQ 目标**:
- 3 个小号(大号/业务号/备用号)就位
- 业务号覆盖 15-20 个跨境群
- 月转化 2-5 个客户(占 M1 总客户 4-10%)
- 渠道结构: TG 60% / 内容平台 35% / QQ 群潜群 5%

**关键动作**:
- W1 每天加 ≤ 2 个群(防风控)
- W2-D11 起每天主动 @ ≤ 5 人
- W3 业务号被踢 < 3 次属正常, 立即备用号
- W4 客户标 source=qq, 同步 Notion CRM

**关联**: [[13-v0.8.29-运营SOP-TG+国内]] §10

## 2026-07-03 v0.8.29 SOP: §4 TG 建群 + Channel 细节扩充 (4 节 → 9 节)

**决定**:
1. §4 从原来的 4 节扩充到 9 节(群架构 / Channel / 主群 / 分群 / 置顶 / 链接 / 内容 / 私聊 / 防封)
2. 新增 Step by Step 操作:
   - Channel 5 步建设(创建 / 签名 / 置顶 / 慢速 / 邀请)
   - 主群 7 步建设(创建 / 权限 / Anti-Spam / 关联 / 管理员 / 置顶 / 机器人)
   - 分群 3 类型(IG 专项 / TikTok 专项 / 代理内群)
3. 新增 4 套置顶消息模板(主群欢迎 / 主群价格 / IG 专项 / 代理内群)
4. 公开/私密选择逻辑(分 M1-M4 阶段)
5. @username 注册策略(5 个候选 + Premium 解锁 4 位数)
6. 邀请链接生命周期(3 种类型 + 每月重置 SOP)
7. 防封 + 申诉 12 条(频道/群/链接/不同群发言/刷屏/凌晨等)
8. @username 被占 5 招 + 主群/频道被封 5 步应急

**W1-D1 拆分**:
- 原: "建 TG 频道 + 主群 + 拉 20 种子"
- 新: 4 行任务
  - D1-TG1: 5 步建 Channel + 关联主群 + 2 置顶
  - D1-TG2: 7 步建主群 + 权限 + Anti-Spam + 关联 + 3 admin + 2 置顶
  - D1-TG3: 拉 20 种子(5 自 / 5 朋友 / 10 老客)
  - D1-QQ: 申请 2 个 QQ 小号

**关联**: [[13-v0.8.29-运营SOP-TG+国内]] §4 (1+1+N 群架构 + Channel/主群/分群细节)

## 2026-07-03 v0.8.30: Analytics 埋点 + Admin 监控页面上线

**决定**:
1. D1 新增 2 张表:`page_views`(每次请求)+ `funnel_events`(关键事件)
2. 新增 `src/middleware/tracking.ts` 中间件,异步写 page_views(失败不影响主流程)
3. 关键路径同步写漏斗事件: 7 个 view 事件 + 1 个 purchase_attempt
4. 新增 `/admin/analytics` 页面: 6 KPI + 7 步漏斗条 + 14 天 PV 折线 + Top 路径 + Top bundle + 国家/语言分布
5. admin 首页加"📊 访问分析"卡片入口

**关键设计**:
- session_id 用 `tp_sid` cookie 维持(30 天),与 JWT `tp_session` 并存
- bot / human 简单 UA 字符串判断
- 跳过 `/admin` / `/_*` 路径(免污染)
- D1 写: 1k PV/天 = 200KB/天(免费层 10GB 够 10+ 年)
- 漏斗不上传 meta(暂只记 event 名,后续可加 amount_cents / service_id)

**验证**(11 次 curl 模拟 + admin cookie):
- PV=11 / UV=11 / 新注册=1 / 新充值=0 / 新订单=0
- 漏斗 7 步全渲染 + 进度条
- 弱环节识别: "价格→开荒包 流失 100%"
- 国家 CN(11)/ 语言 zh-CN(11)

**关联**: [[14-Analytics埋点+admin页面]] 完整部署记录

## 2026-07-03 v0.8.31: 告警 + TG Bot 推送

**决定**:
1. 新建 `@crm_exprt_bot` (TrendPanel Alert Bot), token 已注入 wrangler secret
2. 新增 `src/lib/tg_bot.ts` 封装 + `src/cron/alert_check.ts` 4 类检查
3. 新增 cron `0 1 * * *` (北京 09:00) 自动推送
4. 新增 `/admin/alerts` 页面 + 3 个 API
5. 新增 D1 表 alert_runs + services.updated_at 列

**4 类告警**:
- UV 突降 (昨日 < 7日均 × 50% 且 7日均 > 10)
- 5xx 错误率 (24h 内 5xx > 5% 且请求 > 100)
- 7 天无新订单
- 每日 cron 状态报告 (failover / probe / sync)

**部署 Version**: 06e3c2d0-df7f-499f-acdd-0c5d7085b9f9
**TG_BOT_TOKEN**: 已注入 (8699758118:AAEUIiBh46roPG4phEmmlvUQTZfGNUF4MZ0)
**TG_ALERT_CHAT_ID**: 待用户给 bot 发消息后获取

**chat_id 获取流程**:
1. 用户在手机给 @crm_exprt_bot 发 /start 或 hi
2. 调 `GET /api/bot/get-chat-id` 拉 chat_id
3. 注入 TG_ALERT_CHAT_ID secret
4. 测试消息 + 手动触发告警

**关联**: [[15-告警+TG-Bot推送]] 完整部署记录

## 2026-07-03 v0.8.31 hotfix: chat_id 注入 + 4 类告警实跑成功

**过程**:
1. 用户给 @crm_exprt_bot 发 /start → 拿 chat_id 8359325541
2. wrangler secret put TG_ALERT_CHAT_ID 8359325541 ✅
3. 测试消息发送成功(用户收到 "🧪 TrendPanel 告警测试消息")
4. 手动触发 /api/admin/cron-alerts: triggered=1 (7 天无新订单)
5. 用户 TG 收到 2 条: 1 条告警 + 1 条每日 cron 报告

**修复 1 个 bug**:
- `src/cron/alert_check.ts` 查 upstream_probes 用错字段名(ts → probed_at)
- 修后 typecheck + 部署 v9df57ad7 → fbaa7de9

**alert_runs 落库**:
- daily_report / 7 天无新订单 / 5xx 错误率 / UV 突降 (4 行)
- 全部 2026-07-03 22:55:31 (UTC+8)

**最终部署**: Version fbaa7de9-5858-4136-8d13-272ea97f7369
**admin 首页**新增"🚨 告警中心 →"卡片

**关联**: [[15-告警+TG-Bot推送]] 完整部署记录

## 2026-07-03 v0.8.32: 充值审核 TG 推送

**触发**:用户 "新的充值审核也需要推送"(继 v0.8.31 告警中心 + TG Bot 后扩展)

**决策**:
- 提交 / 审核通过 / 审核拒绝 **3 个事件**全部走异步 TG 推(@crm_exprt_bot, chat 8359325541)
- 异步推送用 `c.executionCtx.waitUntil`,失败不阻塞主流程(下单 + 审核照常成功)
- 凭证长度限制 **200 字符**(prod 是 400,推送用 200 防超 TG 4096 限制)
- **客户不通知**: 当前只推 admin 频道,客户通过 /dashboard 看到状态变化(避免双向推送复杂度)

**范围**:
- 拒绝**未推 reason**(目前 v0.8.32 不带,后续可加,优先做"客户 TG 通知")
- **没做审批风暴防护**: 同 1 笔不会重复审(cron 告警才有类似问题),v0.8.33 再统一处理

**部署**: Version `4f3e5efb-fc58-43fe-9de9-4389de936bef`,3 文件改动(topup.ts / admin.ts / tg_bot.ts 加函数)

**验证**: 端到端 user_id=13 提交 100 CNY → admin 通过 → 6 步全过

**下一步候选**:
- 客户 TG 通知(下单成功 / 充值通过 / 订单完成 / 余额不足)
- 审批风暴防护(debounce + 同 topup 5 分钟内不重发)
- 告警升级(24h 未处理 → 升级到 @samueldenk)

## 2026-07-03 v0.8.33: 5 项优化打包上线 (客户通知 + 防重推 + 告警升级 + 批量审核 + alert_runs 落库)

**触发**: 用户 "1-5 都进行吧" (5 个候选)

**决策**:
- **客户 TG 通知** 走 opt-in: 不强制绑定, 客户主动私聊 `/bind <email>` 才推 (避免 GDPR/隐私问题)
- **审批防重推** 用 `topup_notify_log` 表的 `(topup_id, action, channel) UNIQUE` 约束, `INSERT OR IGNORE` 自动跳过
- **告警升级** 24h-7d 窗口: 老于 7d 不再升级 (避免历史告警反复); 升级后写 `escalated=1` 标记
- **批量审核** 单次 ≤ 100 笔上限, 防 admin 误操作刷爆
- **alert_runs 落库** 补 v0.8.31 漏的(此前只发 TG 没存库, 升级机制依赖这张表)

**拒绝考虑**:
- ❌ 强制客户绑定 (改 opt-in, 客户不绑也能用, 只是不收推送)
- ❌ 验证邮箱所有权 (OTP 太重, TG chat_id 私聊本身有信任)
- ❌ 实时告警升级 (走每日 cron 批量扫, 减少 API call)

**部署**: Version `35c37b3e-67a3-4577-a66d-2505920c87bc` (11 文件改动, +521 行, 0 删)

**验证** (user_id=14 端到端 10 步全过):
- 注册 → /bind → 提交充值 → 审核 → 批量 → 下单 → 余额不足 → cron 告警 → /unbind → webhook /start 全部 ✅
- notify_log 3 行全写入 (submitted admin / approved admin / approved user)
- alert_runs 4 行 (3 告警 + 1 daily_report) + escalated 列存在

**TG Bot webhook 上线 1 步** (待用户执行):
```bash
curl "https://api.telegram.org/bot<TOKEN>/setWebhook?url=https://stackmatrices.com/api/tg/webhook"
```

**未做** (v0.8.34 候选):
- 客户 OTP 验证邮箱所有权
- 客户 /unbind 后老 chat_id 自动清理
- 告警升级支持 Telegram thread 模式 (用同一 message_thread_id)

**关联**: [[17-v0.8.33-客户TG通知+5项优化]] / [[15-告警+TG-Bot推送]] / [[16-充值审核TG推送]]

## 2026-07-03 v0.8.34: 登录/注册健壮性加固

**触发**: 用户 "登陆和注册流程还需要健壮一点, 我刚刚第一次登陆 admin 账号是报错的, 但是不知道报什么错。第二次点击登陆才成功"

**根本原因 (4 类)**:
1. 客户端 catch 只返笼统 "Network error", 不区分 HTTP 状态码 / JSON 解析 / 网络断
2. Cloudflare 5xx/网关返 HTML, `r.json()` 抛 SyntaxError 归到 catch → 显示 "Network error"
3. 没用 `r.ok` 检查, 500 假阳性可能
4. 跳转前没验证 cookie 生效, 首登偶发白屏

**决策**:
- **服务端** try/catch 兜底 + JWT_SECRET 缺失检查 + 邮箱小写化 + 防枚举(同返 bad_credential) + 详细 console.log(ms/ip/user_id)
- **客户端** `r.ok` 检查 + `Content-Type` 检查 + JSON parse 单独 try + 客户端先校验 + 错误码细分 errMap + 跳转前 /api/me 探一次 + 按钮 disable + reset
- **新端点** `GET /api/me` 探 cookie 是否生效(防首登白屏)

**部署**: Version `8d960c9d-8dab-4046-bac9-531e62dd61fe` (1 文件, +161/-47)

**验证** (10 场景全过): 正常登录 / /api/me / 错密码 / 不存在邮箱 / 空字段 / 弱密码 / 无效邮箱 / 邮箱已存在 / 正常注册 / 清理

**关联**: [[18-v0.8.34-登录注册健壮性]] / [[17-v0.8.33-客户TG通知+5项优化]]

## 2026-07-04 v0.8.35: /admin/services 性能优化 (D1 索引 + 分页 + SQL 精简)

**触发**: 用户 "/admin/services 加载卡死, 觉得 D1 数据库也要优化, 索引等"

**根本原因 (3 个瓶颈)**:
1. `SELECT s.*` 全列 — services 表 31 列含 description_long / name_en, 每行 ~1KB
2. 无 LIMIT — 一次返 5122 行 = ~5MB HTML (接近 Worker 1MB 限制)
3. 2 关联子查询 — 对每行扫 service_mappings, rows_read 25609

**决策**:
- **4 个新 D1 索引** (idx_services_cat_sort / idx_services_status / idx_mappings_svc_enabled_pri / idx_platforms_sort), D1 size 6.8MB → 7.3MB
- **SQL 精简** 31 列 → 16 列 (只查 UI 必要字段, description_long / name_en 等大字段不查)
- **分页** (page=N, size=50/100/200, LIMIT/OFFSET)
- **筛选 UI** (平台下拉 / 状态下拉 / 每页大小)
- **不改写为 LEFT JOIN** (D1 SQLite 不支持 window function, 子查询版已 5-7ms 够快)

**部署**: Version `f6f46d2d-8289-44ce-b156-d1f6f6ead19a`

**性能对比**:
| 查询 | 旧 | 新 | 加速 |
|---|---|---|---|
| 旧 SQL (全列+子查询+5122行) | 305ms | — | baseline |
| 新 SQL (16列+子查询+5122行) | — | 117ms | 2.6x |
| 新 SQL + LIMIT 100 | — | 5-7ms | 50x |
| HTML 大小 | 5MB | 149KB | 33x |

**实测页面**: 首次 10s (cold start) / warm 2-3s / D1 查询 < 10ms / 5,122 服务分 103 页

**关联**: [[19-v0.8.35-性能优化-D1索引+分页]]


## 2026-07-06 客户定位调整(重要)

**决策**:目标客户从"原 Salesforce 用户"调整为"**正在用 HubSpot / Zoho / Pipedrive / 销售易 / 纷享销客的中小企业**"。

### 理由
1. **Salesforce 决策周期 6-12 个月**(跨国合规、IT 评估、采购流程),**一年开不了单**
2. **HubSpot/Zoho/销售易/纷享销客用户**:
   - 已经在用 CRM,有付费意愿
   - 痛点明确(贵、臃肿、没 AI、套餐陷阱)
   - 决策周期 1-3 个月
   - 单价中等,易成交(部署费 ¥2-3 万)
3. **Twenty CRM 的差异化卖点正好打这些痛点**:
   - 自托管,数据境内,合规
   - 真正的 AI Agent(不是 chatbot 噱头)
   - 一次性买断,无席位费

### 调整后的客户画像
- **行业**:外贸 / 教培 / 法律 / 财税 / 咨询(知识服务型,数据敏感)
- **规模**:10-100 人,年营收 500 万-1 亿
- **现有 CRM**:HubSpot Free/Starter / Zoho Standard / 销售易 / 纷享销客
- **决策人**:老板本人或运营总监(不是 IT 部门)
- **预算敏感**:年付 1-5 万 CRM 已是上限

### 调整后的视频选题钩子
- 不再: "50 人外贸公司年付 18 万 Salesforce"
- 改成: "你们公司在用 HubSpot / Zoho 吗?这 3 个坑你一定踩过"
- "Zoho CRM 标准版 ¥140/月/人,10 个销售一年 ¥16,800,其实你只用 3 个功能"
- "销售易 1 个席位 ¥4800/年,50 人团队一年 ¥24 万,贵在哪里?"

### 下一步
- 重写 12 条视频选题,核心痛点改为"贵 + 臃肿 + 没 AI"
- 录制前 5 条做 A/B 测试:vs HubSpot / vs Zoho / vs 销售易 各一条

## 2026-07-06 定价大调整(关键)

**决策**:部署费从 ¥3 万压到 **¥5,800-19,800 三档**,DeepSeek AI 不单独收费。

### 之前的问题
- 部署费 ¥3 万 + AI ¥1 万 = 起步 ¥4 万
- 小公司根本不会买(5-20 人团队年付 Zoho 才 ¥1.7 万)
- **一年开不了单**

### 调整后的定价(三档)

| 套餐 | 一次性 | 适合 | 我们毛利 |
|---|---|---|---|
| 🥉 基础版 | ¥5,800 | 5-20 人 SOHO | ¥4,000(70%) |
| 🥈 专业版 | ¥9,800 | 20-50 人成长型 | ¥7,500(75%) |
| 🥇 旗舰版 | ¥19,800 | 50-100 人企业 | ¥14,000(70%) |

**年订阅**(可选):¥1,200 - 8,800/年

### 关键变化
- **DeepSeek 不单独收费** — 客户自己申请 API key,一年 ¥50
- **AI 集成包含在部署费** — 不是增值服务,是标配
- **样板客户 5 折**(¥2,900)— 拿 case study + 截图 + 视频
- **5 个工作日交付**(不是 1 个月)— 小公司决策快,服务也要快

### 单客户 LTV(3 年)
- 基础版:**¥11,400**(首单 + 订阅 + 增值)
- 专业版:**¥25,600**
- 旗舰版:**¥50,000+**

### 12 个月目标
- 30 个客户,年流水 **¥30-50 万**

### 下一步
- 重写第一条视频脚本:钩子数字改为"Zoho 16,800 vs 我们 ¥8,250"
- 1v1 话术按 ¥5,800 起步价算账
- 私域 SOP 的"客户分档"按 ¥5,800 / ¥9,800 / ¥19,800 调整

---

## 2026-07-06 · Zoho 痛点视频 PPT 全套完成

### 关键动作
- 重写英文版 `build.py` 为中文版,12 页深色科技风 PPT
- 6 个布局 bug 修复(¥16,800 / ¥140 巨号溢出、page 10 价格溢出、page 11 引号显示成三角、page 12 红条压页眉、page 7 绿块文字重叠)
- 全部 12 页 PNG 验证中文渲染 OK
- 演讲稿 12 段,适配 TTS,总时长 2:10
- 录制 SOP(OpenScreen + 剪映)+ 4 平台发布策略

### 关键决策
- **PPT 模式确认有效**:深色科技风 + 巨号数字钩子 + 客户原话金句,12 页信息密度合适
- **演讲稿必须分页写**:每页独立段落,配 `[停顿]` `[长停]` 标记,TTS 才能听懂节奏
- **录制首选 Keynote** 打开 PPTX(避免 LibreOffice 翻页黑屏)

### 下一步(本周末前)
- 录制 PPT 空镜(20 分钟)
- TTS 配音(20 分钟)
- 剪映拼接 + 字幕(40 分钟)
- 抖音首发,验证 5 秒完播率

## 2026-07-06 opspilot.cc 第一条视频 v2(16 页 4 段)


### 决策
- **页数**:12 → 16,新增 4 页 opspilot.cc 能力演示(部署/对接/AI/数据流)
- **视觉**:从"全 16 页米白单段"改成"4 段不同色调",视频有节奏
  - A 痛点(米白)+ B 对比(薄荷绿)+ C 能力(**深色技术感**)+ D 证言(暖米)
  - 段 C 深色是有意为之(模拟录屏/控制台),其他 3 段浅色拉开节奏
- **品牌**:TrendPanel → opspilot.cc(全小写),所有页面统一
- **录屏占位**:页 10/11/12 预留 `recordings/*.mp4`,录完后 `sed` 替换
- **价格**:基础 ¥9,800 / 专业 ¥22,416(企业版面议),按 [[11-小公司合理定价与成本测算]] 落实

### 交付物
- `~/Documents/Obsidian Vault/项目/4-CRM私有化部署/13-zoho-pain/`
  - v2:16 页 PPTX + PDF + overview + 16 notes + 脚本
  - v1 备份保留(`zoho-pain-cn.v1.*` + `page-cn-*.png.bak`)
- `~/Documents/CRM/ppt-output/01-zoho-pain-pptmaster/` (Codex 源)
- `make-video.sh` + `generate-tts.sh` 一键混剪

### 下一步
1. 用户录 3 段 mp4
2. 跑 TTS + 混剪脚本
3. 剪辑 + 加字幕 + 发布

### 关联
- [[12-PPT模式视频方案]] · [[11-小公司合理定价与成本测算]]
- [[13-zoho-pain/README|13-zoho-pain v2 详情]]

## 2026-07-08 决策 — 启动"离线会记"M0.5(改造第 1 周)

**触发**:
- 商标查询"离线会记"4 类无冲突(TM 库干净)
- 用户精力可分 50% 给本地软件
- Meetily 上游画像清晰(MIT / 21k stars / 中文真空 / 二次商业零法律障碍)

**决定**: 启动会记 M0.5(基于 Meetily fork),不再等 SMM M1 跑通

**里程碑**:
- W1(7/8-7/14) 屏蔽云端 + CSP 重写 + 跑通 build
- W2(7/15-7/21) 国产 ASR(SenseVoiceSmall via sherpa-onnx)接入
- W3(7/22-7/28) 离线授权 + 录音告知 + 商标申请 + Apple 开发者
- W4(7/29-8/4) MVP 内测 50 种子用户
- W7(8/19-8/25) 公开商业发布
- 9/30 累计 ¥5000-30000 流水

**预算**: ¥5000 起步
- 商标 9 类 + 42 类:¥540(9-12 月审核)
- 域名 lixianhuiji.cn/.com:¥160
- Apple 开发者账号:$99(≈¥700)
- Win EV 代码签名:¥3000-8000(可缓到 W6+)

**首版定价**:
- 单机买断 ¥399 / Pro 年费 ¥299 / 行业术语插件 ¥99-149 / 教育版 ¥199

**KPI**:
- 30 天 ¥5000 流水
- 90 天 ¥30000 流水
- 首年 ¥35-65 万净利(保守)

**砍掉**(明确边界):
- 不做云端 / SaaS / 移动端 / PWA / Web 版 / 订阅 / OEM / 开源二次分发

**风险**:
- SMM 精力被分 → SMM W4 收尾后再集中到会记
- 上游 API breaking change → 只拉不推,跟踪 devtest 分支
- 国产 ASR 集成工作量超预期 → 优先用 sherpa-onnx(已有 Rust binding),不用 FunASR 原生

**关键依赖**:
- 上游:Zackriya-Solutions/meetily (MIT, 21k stars)
- 国产 ASR:FunASR SenseVoiceSmall (MIT, 234MB)
- 本地 LLM:通义千问 Qwen2.5-7B-Instruct-GGUF (Apache 2.0, 4.7GB)
- 集成框架:sherpa-onnx (Apache 2.0, Rust binding)

**关联文档**: [[项目/N-离线会记/01-调研-上游Meetily+商业可执行版]]

## 2026-07-09 00:35 — W1.9: LANGUAGE_PREFERENCE 强制 zh

**触发**: 用户截图显示转录乱码(`9刀之间 具体流杀...9刀K 还能管2呢`),摘要报空
**根因**:
- meetily v0.4.0 上游写死 `LANGUAGE_PREFERENCE = "auto-translate"`,老用户 ConfigContext 默认 `auto`
- Whisper small + auto-detect 模式,遇到中文强行吐 K/9刀/打发 这种占位符
- Whisper small 中文 CER 本身 ≈ 25%,与默认 auto 模式叠加雪崩

**决定**:
- commit `72042fd fix(w1.9)`:LANGUAGE_PREFERENCE 默认改 zh,ConfigContext 启动清掉老 auto 缓存
- DB 模型:**`ggml-large-v3-turbo-q5_0.bin`** 547MB,中文 CER 8-12%(临时方案)
- 模型已从 hf-mirror.com 拉到 `~/Library/Application Support/cn.lixianhuiji.app/models/`

**临时方案**:
- tauri dev 重启后, 中文转写应正常(可能少量专有名词误差)
- 实在不行:重新录一次,或换 medium 模型

**W2 国产 ASR 代码**:
- 已落袋 `outputs/patches/w2-sherpa-asr/`
- 269 行 sherpa_provider.rs(Paraformer + SenseVoice 双 backend)
- `apply.sh` 一键 patch + install scripts + DB migration
- 等明天 cargo check(预计 5-10 min)

**关键调优**:
- FunASR-Nano 0.8B ONNX 不可用 → 改 **SenseVoice INT8**(同生态,228MB)
- UI 上的 "FunASR-Nano" 名字保留(更熟),后端走 SenseVoice

## 2026-07-09 11:35 — W1.13 白屏彻底根因 + release binary 启用

**触发**: 用户开 meetily GUI 一直看到白屏,试 next.js dev / python http.server 都被 Tauri webview 弄死

**根因**:
- meetily 当前 binary 是 `target/debug/meetily`,从 `cargo build -p meetily` 编译
- Tauri 2 配置:`devUrl: http://localhost:3118`,`frontendDist: ../out`
- **debug build 的 binary 启动时优先走 devUrl,加载 `http://localhost:3118`**
- 用户没装 cargo-tauri-cli,所以 `cargo tauri dev` 跑不了 → beforeDevCommand 不会自动启 next dev
- 我手动启 next dev,Tauri webview 抢资源 + port 冲突,把 next 弄死
- 改 python http.server 也被杀,因为 Tauri webview 拿不到页面就 spin 死锁

**决定**:
- `cargo build --release -p meetily` → `target/release/meetily` 61MB
- release binary 走 frontendDist (../out 静态文件),不依赖 dev server,无白屏
- 完整 release build 4.5 分钟 (screen tmux 隔离运行,避免 shell 被工具边界 kill)
- `start-meetily.sh` 改为 detect / 优先 release binary
- Obs 已同步新版本脚本

**下一步**:
- 用户测试 release binary 是否能正常显示 UI
- 若 OK,今晚录音 30 秒中文 + 摘要测试就能跑了
- 后面再做 W2 (sherpa-onnx 国产 ASR)

### W1.15 fix(2026-07-09 13:08) — localWhisper 录音卡死
**Bug**:`handleRecordingStart` 只调 `parakeet_init`+`parakeet_has_available_models`,用户切到 `localWhisper large-v3-turbo-q5_0`(硬盘 547MB Ready)录音按钮永远弹 "Transcription model not ready"。

**根因**:`useRecordingStart.ts:53-76` 函数只检测 Parakeet,不看 `transcriptModelConfig.provider`。

**修复**(`frontend/src/hooks/useRecordingStart.ts`,+18/-7):
```ts
const provider = transcriptModelConfig?.provider ?? 'parakeet';
if (provider === 'localWhisper') {
  await invoke('whisper_init');
  return await invoke<boolean>('whisper_has_available_models');
}
// default + parakeet 走原路径
```
- `checkIfModelDownloading` 同样按 provider 选 `whisper_get_available_models` | `parakeet_get_available_models`
- 函数名保留 `checkParakeetReady`/`checkIfModelDownloading`(最小 diff),内部 provider-aware
- 2 个 useEffect deps 加 `transcriptModelConfig`(React lint)

**Tauri 命令**(已存在,无需 Rust 端改动):
- `whisper_init` / `whisper_has_available_models` / `whisper_get_available_models` (`whisper_engine/commands.rs:55,222`)

**Typecheck**:`tsc --noEmit` 仅 1 个无关 `bun:test` 测试文件 error。`next build` 成功,新 chunk 含 `whisper_has_available_models` 字符串。

**重 build**:
- `frontend/out/index.html` 13:04
- `target/release/meetily` 13:07
- background PID 17202(screen meeti)

**验证**:`log:` `discover returned 12 models: [("large-v3-turbo-q5_0", "Available")]` ✓

**下次点录音**:应不弹 "Transcription model not ready",直接 startRecording。

### W1.16 fix(2026-07-09 13:25) — 录音跑通,识别质量差
**验证录音**:Meeting 2026-07-09_13-20-55 转录输出流可见,说明 W1.15 修复成功,localWhisper `large-v3-turbo-q5_0` 模型在录音路径上工作正常。

**识别质量(用户反馈"差距太远了")**:
- [00:06] "你能听见我说什么吗?" — 正确
- [00:41] "你怎么听不见我在说什么呢" — 正确
- [00:54] "他结婚给我什么事啊" — 听错
- [00:57] "我是为了绣鹅嫂子为了孩子们" — 人名识别错("绣鹅"应为"X 嫂")
- [01:06-11] 多句口语/语气词识别混乱

**根因**:
1. `large-v3-turbo-q5_0` 在中文 **口语/语气词/方音/人名** 上 CER 8-12%,专业场景下 20-30%
2. 默认 VAD 静音切断太敏感:每 5-7 秒一段,对应 VAD `min_silence_ms` 偏小(默认 ~500ms)
3. 无 speaker diarization
4. Whisper 初始化没强制 `language=zh` 提示

**建议方案(按 ROI)**:
- **P0(30 min,今晚可做)** — 调 Rust `whisper_engine.rs` VAD 参数:`min_silence_ms` 1000→2000,加 `language=zh` 默认 prompt
- **P1(明天,W2 提前)** — 集成 sherpa-onnx Paraformer-zh-int8,中文 CER 5-8%,比 whisper q5_0 高 2-3 倍;模型已在 `~/Library/.../models/sherpa/paraformer-zh-int8/model.int8.onnx` 217MB,但**未接入**
- **P2-P3(下周到 W2)** — speaker diarization + 自动分段摘要

**决策**:等用户回复选 P0 or P1 路线

### W2.0 fix(2026-07-09 13:55) — 接 sherpa-onnx Paraformer/SenseVoice
**触发**:用户测试音频 "你好我是王威..."(2 分 7 秒)对比 Paraformer(正确) vs Whisper q5_0(严重幻觉,生造"绣鹅嫂子结婚孩子")

**方案选型**(避开 W2 patch 里 5-10 分钟 cargo build 加 sherpa-rs 子模块):
- 放弃 `sherpa-rs` crate(git 子模块 + 编译依赖重)
- 改用 **subprocess 调用本地已装的 sherpa-onnx-1.13.4 Python wrapper**
- 模型已下载到 `~/Library/.../models/sherpa/{paraformer-zh-int8,sense-voice-zh-int8}` 217MB / 228MB

**新增 4 文件**:
1. `frontend/src-tauri/scripts/sherpa_asr.py`(110 行)
   - stdin/stdout 行式 JSON 协议
   - 接收 `{model,audio_b64,sample_rate}` 返 `{text,confidence,duration_ms,load_ms}`
   - 模型 lazy-load + 复用(冷启 1s,热调 50ms)
2. `frontend/src-tauri/src/audio/sherpa_daemon.rs`(130 行)
   - `SherpaDaemon` 长寿命子进程,Mutex 保 stdin/stdout
   - `global()` + `transcribe_blocking()` API
3. `audio/mod.rs` 注册 `pub mod sherpa_daemon`
4. `frontend/src/hooks/useTranscriptionModels.ts` 加 2 个 sherpa 静态选项(Paraformer/SenseVoice)

**改 2 文件**:
- `audio/retranscription.rs`: `start_retranscription` 和 `run_retranscription` 加 `use_sherpa` 分支
  - sherpa 路径:**跳过 VAD 段级转录,直接对完整 16k mono 跑 1 次**,返回单段整文字覆盖(中文 SOTA 模型本来就适合整段 inference,VAD 切分反而切碎语义)
  - 前置 clone 一份 `sherpa_audio_samples` 避开 VAD 闭包 move
- `Cargo.toml`: 加 `base64 = "0.22"`(0.22 API: `STANDARD.encode(&bytes)`)

**Patch 验证**:
- `cargo check -p meetily` 0 error(7.4s)
- `cargo build --release -p meetily` 0 error(2m 27s, 66MB binary)
- Python daemon standalone test:Paraformer 4.3s cold / 3.8s warm

**用户体验路径**:
1. 打开离线会记 → 已有 meeting(录音 + Whisper 转录)
2. 点 "Enhance" 按钮 → 弹 RetranscribeDialog
3. Model 下拉里多 2 项:
   - 🐉 Paraformer-zh INT8 (国产 · 中文 SOTA) — 217MB
   - ✨ SenseVoice-zh INT8 (Pro · 多语种/情感) — 228MB
4. 选 Paraformer → Start → 后端:
   - 调 sherpa daemon → Paraformer-zh 跑完整 16k mono
   - 返回 {"你好我是王威 你能恨谁恨谁啊 你谁呀..."}
   - 写入 transcripts.json 覆盖原 Whisper 输出
   - 前端 streaming 进度条(35% 启动 → 50% 调用 → 75% 写入 → 80% 完成)
5. 用户切回 meeting,文字立刻是 Paraformer 正确结果

**对比基线**(用户这段 127s 音频):
- Whisper large-v3-turbo-q5_0: "你能听见我说什么吗? 你怎么听不见我在说什么呢 他结婚给我什么事啊 我是为了绣鹅嫂子为了孩子们 ..." (CER 高,幻觉严重)
- **sherpa Paraformer-zh-int8**: "你好我是王威 你能恨谁恨谁啊 你谁呀 我是谁呀 你怎么听不见我在说什么呢" (CER <10%,准确识别人名/口语)

**风险/兜底**:
- daemon spawn 失败 → `Err(e)` → 走原 Whisper 路径(`warn!("Sherpa daemon 失败: {}", e)`)
- base64 大文件(127s = 2MB 16k f32 = 2.7MB base64)subprocess 5s 内 OK
- daemon 不重用,logout 自动 kill

**未做(下次)**:
- Real-time 流式录音接 Paraformer(当前只支持 retranscription 批量重转录)
- 多个 speaker diarization
- 模型自动下载逻辑(目前需手动保持模型文件存在)
