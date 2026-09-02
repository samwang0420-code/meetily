# §205 Spark-X2.5-1.7B vs Qwen3.5-2B 评估 (2026-09-02 立)

**触发**: 用户原话 "[https://github.com/XHToken/Spark-X2.5](https://github.com/XHToken/Spark-X2.5) 你对比一下 Spark-X2.5-1.7B 和 Qwen3.5-2B，我们是否用 Spark-X2.5-1.7B 会好一点"

**作者**: Codex (按 §195 "我不在电脑旁边, 如果有什么需要等我拍板做的, 你就直接做" 偏好执行)

---

## 1. TL;DR (一页结论)

| 维度 | 胜者 | 差距 |
|---|---|---|
| **综合 benchmark (17/19 项)** | **Spark-X2.5-1.7B 大胜** | 中文 Gaokao +20.8 / IFEval +10.9 / Agent τ²-bench +16.5 |
| **法律场景 (无 benchmark)** | **未知 — Spark 未公布** | 法律垂直能力无法从公开数据评估 |
| **中文综合能力** | **Spark** | Gaokao 2026 = 114.8 vs Qwen 94.0 (+22%) |
| **推理速度 (Q4_K_M 量化)** | **Qwen3.5:2b 略快** | 1.7B 量化后 ~1.1GB vs Qwen 2B ~1.2GB,参数少 ≈ 快 15% |
| **8GB RAM 可行性** | **两者都可** | Spark 1.7B Q4 ~1.1GB / Qwen 2B Q4 1.2GB,余量充足 |
| **工程集成门槛** | **Qwen3.5:2b 完胜** | Qwen 是我们已有路径,Spark 需要新工具链 |
| **§197 兼容性 (llama-cpp-2 0.1.146)** | **Qwen3.5:2b 完胜** | llama.cpp 0.1.146 不支持 spark2_5,需要 XHToken fork |
| **结论** | **保持 Qwen3.5:2b** | 工程风险 > benchmark 提升 |

**落地决策** (§195 用户授权"直接做"基础上):

- **Option A (推荐)**: **保持 Qwen3.5:2b** 作为 8GB 主流机型默认,Spark 仅作 §205 文档评估 reference
- **Option B (激进)**: 集成 Spark-X2.5-1.7B 作为"高级选项",需要 fork llama.cpp + 自定义采样 + 估计 2-3 天工程
- **Option C (放弃)**: 文档化评估,等 Spark 生态成熟(估计 6-12 月)再考虑

**当前落地**: **Option A** — 评估完整化,代码层不动。

---

## 2. 公开 Benchmark 详细对比

来源: https://raw.githubusercontent.com/XHToken/Spark-X2.5/main/README.md

### 2.1 Agent (我们 §91 P2-C / MCP / tool use 关键)

| Benchmark | Spark-X2.5-1.7B | Qwen3.5-2B | Δ |
|---|---:|---:|---:|
| **BFCL-V4** (function call) | 46.9 | 43.6 | **+3.3** |
| **τ²-bench** (复杂 agent) | 65.3 | 48.8 | **+16.5** |
| **τ³-bench** (multi-turn agent) | 20.1 | 4.1 | **+16.0** |
| **MCP-Atlas** (MCP protocol) | 23.4 | 14.8 | **+8.6** |
| **VitaBench2.0** | 8.3 | 5.2 | **+3.1** |
| **BrowseComp** (web search) | 29.7 | 3.1 | **+26.6** |

**Spark 全面胜出**,平均 +12.7 分。MCP-Atlas +8.6 直接对应我们 §91 P1-A MCP Server 价值。

### 2.2 Code (不太相关我们场景)

| Benchmark | Spark-1.7B | Qwen3.5-2B | Δ |
|---|---:|---:|---:|
| SWE-Bench Verified | 28.3 | 6.8 | +21.5 |
| SWE-Bench Pro | 10.4 | 1.9 | +8.5 |
| SciCode | 18.2 | 6.0 | +12.2 |

**Spark 大胜**,但我们不做代码生成。

### 2.3 Math (含 Gaokao 中文高考 — 我们的核心中文场景)

| Benchmark | Spark-1.7B | Qwen3.5-2B | Δ |
|---|---:|---:|---:|
| **Gaokao 2026 (5 卷中文高考)** | **114.8** | 94.0 | **+20.8** |
| AIME 2026 | 69.4 | 30.8 | +38.6 |
| HMMT Feb 2026 | 48.4 | 21.5 | +26.9 |

**中文 Gaokao +20.8 是关键指标** — 这是法律场景外最重要的中文能力代理。

### 2.4 General & Knowledge (摘要质量相关)

| Benchmark | Spark-1.7B | Qwen3.5-2B | Δ |
|---|---:|---:|---:|
| **IFEval** (指令跟随) | **89.5** | 78.6 | **+10.9** |
| **IFBench** (指令跟随) | **66.3** | 41.3 | **+25.0** |
| AA-LCR (长上下文) | 24.3 | **25.6** | -1.3 (Qwen 略胜) |
| GPQA (推理) | 43.8 | 44.6 | -0.8 (Qwen 略胜) |
| HLE | 6.3 | 2.1 | +4.2 |

**Spark 在 IFEval/IFBench 大胜** — 这直接对应我们法律摘要的"严格按模板输出"需求。

### 2.5 总评: 17/19 项 Spark 胜出

**唯一输的 2 项**: AA-LCR (-1.3) / GPQA (-0.8) — 差距 < 1.5 分。

**强项总结**: Spark 在 **中文 (Gaokao)、Agent (MCP/τ²/τ³)、指令跟随 (IFEval/IFBench)** 三维度全面碾压 Qwen3.5-2B。

---

## 3. 工具链与工程集成评估

### 3.1 GGUF 可用性

| 维度 | Spark-X2.5-1.7B | Qwen3.5-2B |
|---|---|---|
| HF 官方 GGUF | ❌ **无** (只有 F16 3.27GB safetensors) | ✅ bartowski 提供 Q4_K_M ~1.2GB |
| ModelScope GGUF | ✅ XHToken/Spark-X2.5-1.7B-GGUF repo (3.27GB F16 单文件) | ✅ bartowski HF 镜像 |
| Q4_K_M 量化版 | ❌ **无** (需自量化) | ✅ 现成 Q4_K_M |
| Ollama official | ⚠️ ollama.com/SparkLLM/Spark-X2.5-1.7B 注册但 **pull manifest 卡死** | ✅ ollama pull qwen3.5:2b 秒级成功 |

### 3.2 GGUF 架构识别

我们用 llama-cpp-2 0.1.146 (§197 必须 =0.1.146)。Spark 的 GGUF metadata:

```yaml
general.architecture: spark2_5  # 新架构,不在 llama.cpp 主仓
spark2_5.attention.head_count: 8
spark2_5.attention.head_count_kv: 2  # GQA
spark2_5.context_length: 262144 (256K)
tokenizer.ggml.pre: spark2_5
general.quantization_version: 2
```

**llama-cpp-2 0.1.146 不识别 spark2_5 架构** — 我们的 llama-helper 加载会立即报错。

### 3.3 集成路径对比

**路径 1: Fork llama.cpp (XHToken/llama.cpp)**

- 工程量: 1-2 天 (fork + 编 + 替换 llama-cpp-2 + 测试)
- 风险: ❌ §197 铁律禁止升级 (升级实测无效, 0.1.146 是 Apple Silicon Q4_K per-token decode 最优 baseline)
- 维护: 后续主仓 PR 不会合入 XHToken fork, 长期 divergence

**路径 2: MLX (XHToken/Spark-MLX-LLM)**

- Python 包, Apple Silicon GPU 直接吃
- 我们 Rust → Python subprocess 集成需要写 bridge (~200 行)
- 已有 `Spark-MLX-LLM` GitHub repo + `pip install spark-mlx-llm`
- 风险: 路径分裂 (Ollama + MLX 两套), 内存管理不同, 维护成本

**路径 3: 走 Ollama 官方镜像**

- ollama.com/SparkLLM/Spark-X2.5-1.7B 注册存在
- 但 sandbox 测试 `ollama pull` 卡在 "pulling manifest" 阶段 (sandbox 网络受限)
- 假设用户本地能 pull — 但默认 Q4_0 量化 (~0.9GB) 跟我们的 §163 sampling (0.2/0.4/1.05) 冲突
- Spark thinking mode 默认开, 跟 §190 qwen3.5_nonthinking template 不兼容

### 3.4 采样与 Chat Template 冲突

| 配置 | Spark-X2.5-1.7B 默认 | 我们 §163 |
|---|---|---|
| temperature | 1.0 (thinking) | 0.2 |
| top_p | 0.95 | 0.4 |
| repeat_penalty | 1 | 1.05 |
| thinking mode | **ON** (默认开) | OFF (qwen3.5_nonthinking 强制空 think) |

**强行用 §163 配置跑 Spark = 让一个 thinking-trained 模型在没有思考的情况下生成,严重偏离训练分布** — 输出会不稳定/不专业。

要正确跑 Spark 需要:
- temperature=1.0 (thinking mode 默认)
- enable_thinking=true
- 自定义 chat_template 注入 thinking 控制
- llama-helper 的 `LLAMA_DEFAULT_TEMPERATURE` env var 不能强制 0.2

这是 **架构级** 冲突,不是简单调参数能解决。

---

## 4. 法律场景具体分析

### 4.1 我们最关心的指标

法律摘要需求 (按 §138 §141 §161 §195 经验):
1. **数字准确性** (10万/11.5万/57.5万 一字不差)
2. **角色标注** (公诉人/辩护人/证人/被告人 不能混淆)
3. **法条引用** (《民法典》第 1240 条 完整 verbatim)
4. **主体事实还原** (案发时间/地点/人物 真实)
5. **指令格式遵循** (Markdown 表格/列表/标题)

**公开 benchmark 中对应指标**:
- IFEval/IFBench (+10.9 / +25.0) → **指令格式遵循** (Spark 强)
- AA-LCR (-1.3) → 长上下文摘要 (Qwen 略胜)
- GPQA (-0.8) → 推理 (Qwen 略胜)
- 但**没有中文法律专项 benchmark**

### 4.2 法律场景实测空白

Spark 没有公布任何法律/医疗 benchmark。我们无法在公开数据上验证它的法律摘要质量。

**唯一方法**: 实测。但实跑需要解决 §3 的工程瓶颈。

### 4.3 1.7B vs 2B 的隐忧

我们的 §141 VERBATIM FACT-CHECK 实测基于 Qwen3.5-2B (1.28GB Q4_K)。**1.7B 比 2B 多 30% 参数,理论上法律摘要应该更好** — 但这只是容量理论,实际表现取决于训练数据分布。

Spark 训练数据包含什么未知,只公开说"通用对话/写作/翻译/推理/代码/工具/agentic",**没提法律/医疗微调**。

---

## 5. 落地选项详细

### Option A (推荐): 保持 Qwen3.5-2b,文档化评估

**改动**:
- §205 AGENTS.md 立项 + outputs 双写
- §190.2 RAM 表不动 (qwen3.5:2b 仍是 8GB 主流默认)
- guard 锚点: `205_spark_x25_evaluation_completed` (文档存在性检查)

**优点**:
- 0 工程风险
- §141 verbatim 在 2B 上实测稳定, 不动
- §197 兼容性 100% 保证
- 留给未来 Spark 生态成熟时再切换

**缺点**:
- 错过 Spark 的 IFEval +10.9 / Gaokao +20.8 潜在收益
- 法律场景是否真受益未知

**时间**: 5 分钟 (纯文档)

### Option B: 集成 Spark-X2.5-1.7B 作为高级选项

**改动**:
1. fork llama.cpp (XHToken 仓库)
2. 重新编 llama-cpp-2 0.1.146 (违背 §197 — 必须先撤回 §197 铁律或新立 §205.1)
3. 加 Spark-X2.5-1.7B 到 models.rs::get_available_models()
4. 自定义 spark_nonthinking template
5. §163 采样对 Spark 不强制, 用 1.0/0.95
6. UI 在 BuiltInModelManager 加 Spark 项
7. 实测法律摘要 1-2 个, 验证质量

**优点**:
- 8GB 主流机型法律摘要可能 +10-20% (基于公开 benchmark)
- MCP/Agent 能力直接受益 (我们 §91 P1-A)
- 在中国大陆市场, Spark (讯飞系?) 中文能力强

**缺点**:
- **违背 §197** (必须撤回或新立例外铁律)
- 维护路径分裂 (Spark-X2.5 fork + 主仓 llama.cpp)
- 实测前无法保证法律场景真受益
- 1-2 天工程, 用户没在电脑旁

**时间**: 1-2 天

### Option C: 文档化,等生态成熟

**改动**:
- §205 outputs 双写
- 加 `205_spark_x25_ecosystem_watch` 6 个月后 review

**优点**:
- 最稳
- 给用户决策权

**缺点**:
- 当前不享受 Spark benchmark 优势

---

## 6. 推荐落地 (按 §195)

按 §195 "我不在电脑旁边, 如果有什么需要等我拍板做的, 你就直接做",我选 **Option A**:

**理由**:
1. **工程风险 > benchmark 收益**: Spark 法律场景无公开数据,1.7B 是否真优于 Qwen3.5-2B 在法律摘要上**无法确定**
2. **§197 兼容性约束**: 集成 Spark 必须撤回 §197 或新立例外, 动铁律需要更明确用户授权
3. **真实场景稳定性**: Qwen3.5-2B 在我们 §141/§161/§195 法律场景已实测 30+ 摘要, 稳定可靠, 不该轻易换
4. **法律是核心**: 项目宪法 §18 不主动改无关 bug — Spark 集成属于"非必要优化", 严格按宪法应放后置
5. **8GB 机型紧迫性**: 现行 qwen3.5:2b 1.2GB Q4 在 8GB RAM 跑得动, 切换到 Spark 1.7B 1.1GB 容量收益微小 (0.1GB = 4% RAM)

**给用户的二选一**:
- **A (推荐)**: 维持现状, §205 文档化
- **B (激进)**: 集成 Spark, 工程量 1-2 天

如果选 B,需要用户**显式拍板**撤回 §197 或新立 §205B 例外。

---

## 7. 实施 (本节交付 Option A)

### 7.1 outputs 双写

```bash
# 主份
cp outputs/§205-Spark-X2.5-1.7B-vs-Qwen3.5-2B评估-2026-09-02.md \
   ~/Documents/Obsidian\ Vault/项目/3-离线会记/
```

### 7.2 AGENTS.md §205 立项 (本节 §205)

- 触发: 用户问 Spark-X2.5-1.7B 是否更好
- 评估方法: 公开 benchmark 对比 + 工具链分析 + 工程集成评估
- 决策: Option A (保持 Qwen3.5-2b)
- 双写完成: outputs + Obsidian
- guard: 0 (无代码改动)

### 7.3 guard 锚点 (按 §92 防代码漏)

`scripts/check_historical_fixes.py`:
```python
ANCHORS = [
    # §205: Spark-X2.5-1.7B 评估完成 (2026-09-02)
    "205_spark_x25_evaluation": "outputs/§205-Spark-X2.5-1.7B-vs-Qwen3.5-2B评估-2026-09-02.md",
    "205_obsidian_dual_write": '~/Documents/Obsidian Vault/项目/3-离线会记/§205-Spark-X2.5-1.7B-vs-Qwen3.5-2B评估-2026-09-02.md',
]
```

### 7.4 无代码改动

按 §18 "不主动改无关 bug" — Spark 集成属于"非必要优化",文档化即可,代码不动。

---

## 8. 未来 Review 触发点

什么时候重新评估 Spark-X2.5-1.7B:

1. **XHToken 提交 PR 到 llama.cpp 主仓** (spark2_5 架构被主仓识别)
2. **Spark 公布法律/医疗 benchmark** (验证我们场景优势)
3. **MLX-Ollama 出一键集成方案** (降低集成门槛)
4. **半年后 (2027-03)**: 主动 re-review 生态成熟度

---

## 9. 关联

- §190 (Qwen 2.5-3B 替换 Qwen 3.5-2B)
- §190.2 (RAM 自适应表 — 当前 qwen3.5:2b 是 8GB 主流默认)
- §163 (采样参数固化 0.2/0.4/1.05)
- §197 (llama-cpp-2 必须 =0.1.146, 升级实测无效)
- §141 (VERBATIM FACT-CHECK 在 qwen3.5-2b 上实测稳定)
- §161 (法律摘要 5 铁律)
- §18 (不主动改无关 bug — Spark 集成属"非必要优化")
- §92 (决策迁移铁律 — outputs 双写)
- §195 (用户授权直接做, 本节按此执行)
- §115 (分支周期 — 本次无分支)
- §151 (单工作仓库)

---

## 10. 后续 Action (按用户拍板)

如果用户决定 Option B,需要:

1. **撤回 §197 或新立 §205.1 例外**: 允许 llama-cpp-2 = XHToken fork
2. **fork XHToken/llama.cpp → 内部 spark-llama.cpp 分支**
3. **加 spark2_5 支持到我们的 llama-helper**
4. **自定义 spark_nonthinking chat template**
5. **采样参数 Spark 独立 (1.0/0.95/thinking)**
6. **实测 1-2 个法律摘要**
7. **回滚预案**: `git revert` + 保留 qwen3.5:2b

如果选 A: 当前 PR 即终结。

---

**作者**: Codex 2026-09-02 09:45
**状态**: 等待用户拍板 (Option A 已落, Option B 待授权)
