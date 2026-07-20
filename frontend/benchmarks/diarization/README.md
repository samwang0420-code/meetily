# cam++ 说话人分离固定基准

- `two-speaker-legal.wav`: 2 人、6 个交替发言段。
- `four-speaker-medical.wav`: 4 人、8 个交替发言段。
- `segments/*.json`: 每段的真实开始时间、结束时间和角色标签。
- `evaluate.mjs`: 检查人数准确率和按重叠时间匹配后的 speaker 纯度。

运行：

```bash
python3 scripts/run_diar_benchmark.py
node benchmarks/diarization/evaluate.mjs
```

当前阈值 `0.4`；固定基准要求人数全部正确且 speaker purity ≥ 90%。合成音频用于链路回归，不替代最终真实多人会议验收。

一小时生产证据复核：

```bash
node benchmarks/diarization/verify_long_audio.mjs
```

该命令校验已执行的 3619 秒四人长音频报告、原始 848 段输出、耗时/内存记录，以及生产代码的 300 秒保护是否一致。当前结论是长音频会把 4 人拆成 5 人，因此 release 保持超过 300 秒不启用 cam++。
