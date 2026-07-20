# ASR 固定基准集

- 法律诉讼 5 段、医疗会诊 5 段，每段按正常语速约 30–60 秒。
- `legal/*.txt` 与 `medical/*.txt` 是标准文本，不随模型改动。
- 同一录音分别由 SenseVoice 与 FunASR-Nano 转写，结果保存为 `hypotheses/<模型名>/<case-id>.json`。
- JSON 格式支持数组或 `{ "segments": [{ "text": "..." }] }`；TXT 按行视为分段。

运行评测：

```bash
node benchmarks/asr/evaluate.mjs \
  --hypotheses benchmarks/asr/hypotheses/sensevoice \
  --output benchmarks/asr/reports/sensevoice.json
```

核心指标：CER、行业术语召回率、碎片段比例。Nano 只有在同一批录音上 CER 明显更低、术语召回不下降且稳定性通过后才能成为默认模型。

模型准入判定：

```bash
node benchmarks/asr/compare.mjs
```

默认模型切换门槛：Nano 相对 CER 至少改善 10%、术语召回不低于 90%、平均解码耗时不超过 SenseVoice 3 倍。合成音频用于可重复回归；另保留用户真实录音单例作为外部有效性检查。
