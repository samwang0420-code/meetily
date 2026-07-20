#!/usr/bin/env python3
"""
离线会记 sherpa-onnx ASR daemon (W2.1)。

协议 (stdin/stdout 行式 JSON):
  请求: {"id": "abc", "model": "paraformer-zh"|"sensevoice-zh", "audio_b64": "...",
         "sample_rate": 16000, "language": "zh"}
  响应: {"id": "abc", "ok": true, "text": "...", "confidence": 0.95,
         "duration_ms": 1234, "model": "paraformer-zh", "load_ms": 800}

也支持文件模式(给 dry run):
  {"id":"x","model":"paraformer-zh","audio_path":"/tmp/test.wav"}

也支持 ls 模式(给 UI 发现模型):
  {"id":"x","action":"list"}

特性:
- 模型延迟加载,只加载用过的,反复调用复用同一 recognizer
- 自动发现 models/sherpa/ 下子目录(*-int8 or *),只要含 model.int8.onnx + tokens.txt
- 4 个 provider 类型: paraformer / sense-voice / zipformer(预留) / Paraformer-分角色(预留 hotword)
"""
import base64
import json
import os
import sys
import time
import traceback

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sherpa_hotwords import get_hotwords
from itn_post import apply_itn  # v0.6.13+ ITN 后处理
from diar import count_speakers, is_available as diar_is_available  # v0.6.14+ Speaker Diarization

MODELS_ROOT = os.path.expanduser(
    "~/Library/Application Support/cn.lixianhuiji.app/models/sherpa"
)

# lazy globals
_recognizer = None
_recognizer_backend = None


# ----- Model discovery ----- #

def _scan_models():
    """Scan models/sherpa/ for installed model packs.
    Each subdir <name> with 'model.int8.onnx' + 'tokens.txt' counts as a candidate.
    Returns {tag -> {path, kind, label, dir}}."""
    found = {}
    if not os.path.isdir(MODELS_ROOT):
        return found
    for entry in sorted(os.listdir(MODELS_ROOT)):
        path = os.path.join(MODELS_ROOT, entry)
        if not os.path.isdir(path):
            continue
        lname = entry.lower()
        if "funasr-nano" in lname or "funasr_nano" in lname:
            required = [
                "encoder_adaptor.int8.onnx",
                "embedding.int8.onnx",
                "llm.int8.onnx",
                "Qwen3-0.6B/tokenizer.json",
            ]
            missing = [name for name in required if not os.path.exists(os.path.join(path, name))]
            if missing:
                sys.stderr.write(f"[sherpa_asr] WARNING: {entry} 缺 {missing}, 跳过\n")
                continue
            found["funasr-nano-zh"] = {
                "path": path,
                "tokens": os.path.join(path, "Qwen3-0.6B"),
                "kind": "funasr_nano",
                "tag": "funasr-nano-zh",
                "label": entry,
                "size_mb": round(sum(
                    os.path.getsize(os.path.join(path, name))
                    for name in required if name.endswith(".onnx")
                ) / 1024 / 1024, 1),
                "dir": entry,
            }
            continue
        onnx = os.path.join(path, "model.int8.onnx")
        tokens = os.path.join(path, "tokens.txt")
        if not (os.path.exists(onnx) and os.path.exists(tokens)):
            continue
        # detect kind from dir name
        if "sense" in lname:
            kind = "sensevoice"
            tag = "sensevoice-zh"
        elif "spk" in lname or "speaker" in lname or "role" in lname:
            # Paraformer-分角色: same architecture as Paraformer but with spk tokens
            kind = "paraformer"
            tag = "paraformer-zh-spk"
        elif "paraformer" in lname:
            kind = "paraformer"
            tag = "paraformer-zh"
        elif "zipformer" in lname:
            kind = "zipformer"
            tag = "zipformer-zh"
        else:
            kind = "paraformer"  # default fallback
            tag = "paraformer-zh"
        try:
            size_mb = round(os.path.getsize(onnx) / 1024 / 1024, 1) if os.path.isfile(onnx) else 0
        except OSError:
            size_mb = 0
        found[tag] = {
            "path": onnx,
            "tokens": tokens,
            "kind": kind,
            "tag": tag,
            "label": entry,
            "size_mb": size_mb,
            "dir": entry,
        }
    return found


def _list_installed():
    out = []
    for tag, info in _scan_models().items():
        out.append({
            "tag": tag,
            "label": info["label"],
            "kind": info["kind"],
            "size_mb": info["size_mb"],
        })
    return out


# ----- Provider kinds ----- #

def _load_paraformer(path, tokens):
    import sherpa_onnx
    return sherpa_onnx.OfflineRecognizer.from_paraformer(
        paraformer=path,
        tokens=tokens,
        num_threads=4,
        provider="cpu",
    )


def _load_sensevoice(path, tokens):
    import sherpa_onnx
    return sherpa_onnx.OfflineRecognizer.from_sense_voice(
        model=path,
        tokens=tokens,
        num_threads=4,
        provider="cpu",
        language="auto",
        use_itn=True,
    )


# v0.7.0+: FunASR-Nano (官方称 Paraformer-分角色 但底层是 LLM 解码, 支持原生 hotwords)
# 模型组成: encoder_adaptor.int8.onnx + embedding.int8.onnx + llm.int8.onnx + Qwen3-0.6B tokenizer
# 必须有完整 4 件套, 缺一不可 (sherpa-onnx 1.13.4 from_funasr_nano 强制)
def _load_funasr_nano(model_dir):
    """model_dir: 包含 4 件套的目录"""
    import sherpa_onnx
    encoder_adaptor = os.path.join(model_dir, "encoder_adaptor.int8.onnx")
    llm = os.path.join(model_dir, "llm.int8.onnx")
    embedding = os.path.join(model_dir, "embedding.int8.onnx")
    tokenizer = os.path.join(model_dir, "Qwen3-0.6B")
    for required, label in [
        (encoder_adaptor, "encoder_adaptor"),
        (llm, "llm"),
        (embedding, "embedding"),
        (tokenizer, "Qwen3-0.6B tokenizer dir"),
    ]:
        if not os.path.exists(required):
            raise FileNotFoundError(
                f"FunASR-Nano 缺 {label}: {required} (请下载 zengshuishui/FunASR-nano-onnx)"
            )
    return sherpa_onnx.OfflineRecognizer.from_funasr_nano(
        encoder_adaptor=encoder_adaptor,
        llm=llm,
        embedding=embedding,
        tokenizer=tokenizer,
        num_threads=4,
        provider="cpu",
        language="zh",
        itn=True,
        hotwords="",  # 由 transcribe() 时通过 recognizer 重新设置
    )


_KIND_LOADERS = {
    "paraformer": _load_paraformer,
    "sensevoice": _load_sensevoice,
    "funasr_nano": _load_funasr_nano,
}


def _ensure_model(tag):
    """Load sherpa-onnx recognizer (lazy + cache). tag may be 'paraformer-zh', etc."""
    global _recognizer, _recognizer_backend
    if _recognizer is not None and _recognizer_backend == tag:
        return _recognizer, tag

    models = _scan_models()
    info = models.get(tag)
    if info is None:
        # try alias: 任意 sensevoice-* 都映射到首个已装 sensevoice 模型
        # (sensevoice-zh / sensevoice-zh-int8 / sense-voice-zh-int8 等)
        sensevoice_aliases = {
            "sensevoice-zh", "sensevoice-zh-int8",
            "sense-voice-zh", "sense-voice-zh-int8",
        }
        if tag in sensevoice_aliases:
            for t, i in models.items():
                if i["kind"] == "sensevoice":
                    info = i; tag = t; break
        if info is None:
            # 最终 fallback: 任何已装的模型
            for t, i in models.items():
                info = i; tag = t; break
        if info is None:
            raise ValueError(f"no installed model for tag '{tag}'. installed: {list(models.keys())}")

    loader = _KIND_LOADERS.get(info["kind"])
    if loader is None:
        raise ValueError(f"unsupported model kind '{info['kind']}' for tag '{tag}'")

    t0 = time.time()
    # funasr_nano loader 签名是 (model_dir) 单参; 其他是 (path, tokens)
    if info["kind"] == "funasr_nano":
        # path 在 _scan_models 中已被设为 model_dir (整个目录)
        _recognizer = loader(info["path"])
    else:
        _recognizer = loader(info["path"], info["tokens"])
    _recognizer_backend = tag
    load_ms = int((time.time() - t0) * 1000)
    sys.stderr.write(
        f"[sherpa_asr] loaded {tag} ({info['kind']}) from {info['label']}/ in {load_ms}ms\n"
    )
    sys.stderr.flush()
    return _recognizer, tag


# ----- Audio decoding ----- #

def _load_audio(req):
    sr = int(req.get("sample_rate", 16000))
    if "audio_b64" in req:
        import numpy as np
        raw = base64.b64decode(req["audio_b64"])
        arr = np.frombuffer(raw, dtype=np.float32)
    elif "audio_path" in req:
        import soundfile as sf
        import numpy as np
        arr, file_sr = sf.read(req["audio_path"])
        arr = arr.astype(np.float32)
        if file_sr != sr:
            ratio = sr / file_sr
            arr = np.interp(
                np.linspace(0, len(arr), int(len(arr) * ratio)),
                np.arange(len(arr)),
                arr,
            ).astype(np.float32)
        sr = file_sr
    else:
        raise ValueError("no audio_b64 or audio_path")
    if arr.ndim > 1:
        arr = arr.mean(axis=1)
    return arr, sr


# v0.7.0+ L0: 产品专名纠错 (最高优先级, 不依赖 hotwords 是否启用)
# 离线会记 / Meetily 相关的常见 ASR 误识别, 全部硬规则替换.
PRODUCT_NAME_CORRECTIONS = {
    # 离线会记 / Meetily
    "meet ily": "Meetily", "meetiliy": "Meetily", "me etily": "Meetily",
    "米提利": "Meetily", "米提力": "Meetily", "米体里": "Meetily", "米体例": "Meetily",
    "离线会记": "离线会记", "离县会记": "离线会记", "离线会计": "离线会记",
    "李贤慧记": "离线会记", "李县惠记": "离线会记",
    "lixianhuiji": "离线会记", "li xian hui ji": "离线会记",
    "mely": "Meetily", "meli": "Meetily", "会议mely": "Meetily", "会议meli": "Meetily",
    # SenseVoice
    "s voice": "SenseVoice", "sense voice": "SenseVoice", "sencevoice": "SenseVoice",
    "赛斯沃斯": "SenseVoice", "森斯沃斯": "SenseVoice", "森斯沃斯": "SenseVoice",
    "森斯沃斯": "SenseVoice", "森斯瓦伊斯": "SenseVoice",
    # FunASR / Paraformer
    "funasr": "FunASR", "fun asr": "FunASR", "范阿瑟": "FunASR", "范阿斯尔": "FunASR",
    "poweraform": "Paraformer", "paraformer": "Paraformer",
    "帕拉福莫": "Paraformer", "帕拉福墨": "Paraformer", "帕拉佛墨": "Paraformer",
    "巴瑞福莫": "Paraformer", "百瑞福莫": "Paraformer",
    # sherpa-onnx
    "sherpa onnx": "sherpa-onnx", "sherpa-onnx": "sherpa-onnx",
    "舌尔帕": "sherpa-onnx", "舍尔帕": "sherpa-onnx",
    # BlockNote
    "block note": "BlockNote", "blocknot": "BlockNote", "black note": "BlockNote",
    "博客诺特": "BlockNote", "布洛诺特": "BlockNote",
    # Tauri
    "tauri": "Tauri", "陶瑞": "Tauri", "淘瑞": "Tauri", "ttry": "Tauri", "turi": "Tauri", "tari": "Tauri", "tora": "Tauri",
    # Ollama
    "ollama": "Ollama", "欧拉拉": "Ollama", "欧拉马": "Ollama",
    # Whisper
    "whisper": "Whisper", "惠斯珀": "Whisper", "惠斯波": "Whisper",
    # Qwen
    "qwen": "Qwen", "通义千问": "通义千问", "千问": "千问",
    # Paraformer-zh (产品名)
    "paraf ormer": "Paraformer", "pa raformer": "Paraformer",
}


def _apply_hotword_bias(text: str, words: list, threshold: float = 0.6) -> str:
    """v0.6.10+: 热词纠错强化 (多长度滑动 + 同音替换 + 专业词候选 boost)

    三层策略:
      L1. 同音替换 (whitelist) – 永久硬规则, 不走 fuzzy
      L2. 同音替换 (动态)    – 用户 hotword 中含 a 时, 误识的 b→a
      L3. 多长度 fuzzy match  – 5-gram 滚动窗口 + 比 difflib 更稳的 char-level ratio
    """
    if not text:
        return text

    # 标准化
    out = text.strip()
    # 删多余空白
    import re as _re
    out = _re.sub(r"\s+", " ", out)

    # ----- L0. 产品专名纠错 (硬规则, 不依赖 hotwords) -----
    # 离线会记、Meetily、SenseVoice 等产品名不能等热词加载, 必须无条件纠正.
    l0_hits = 0
    for wrong, right in PRODUCT_NAME_CORRECTIONS.items():
        if wrong in out and wrong != right:
            out = out.replace(wrong, right)
            l0_hits += 1

    # ----- L1. 静态同音 whitelist (用户场景枚举) -----
    # 关键术语 ASR 误识别 -> 正确术语. 只在用户 hotwords 列表里出现该词时才替换,
    # 避免误伤普通对话. (扩充于 2026-07-17 W1 P0)
    STATIC_HOMO = {
        # ----- 通用会议 -----
        "与位": "与会", "与会人员": "与会人员", "以与": "与会",
        "议位": "议位", "椅位": "议位",
        "纪要": "纪要", "纪药": "纪要",
        "会议纪要": "会议纪要", "会议记要": "会议纪要", "会议纪药": "会议纪要",
        "表决": "表决", "表诀": "表决",
        "拍板": "拍板", "排板": "拍板",
        "议程": "议程", "议呈": "议程", "一成": "议程",
        "结论": "结论", "结伦": "结论", "解论": "结论",
        "提案": "提案", "提安": "提案",
        "决策": "决策", "绝策": "决策",
        "决议": "决议", "决意": "决议",
        "议题": "议题", "议题": "议题",
        "通过": "通过", "通国": "通过",
        "延期": "延期", "演期": "延期",
        "散会": "散会", "三会": "散会",
        # ----- 通用工程 / 跨行业通用 (技术研发场景常出现, 任何 pack 都会触发) -----
        "微服务": "微服务", "围服务": "微服务",
        "高并发": "高并发", "高并fa": "高并发",
        "前端": "前端", "前段": "前端", "前duan": "前端",
        "后端": "后端", "后段": "后端", "后duan": "后端",
        "数据库": "数据库", "数ju库": "数据库", "数距库": "数据库",
        "接口": "接口", "接kǒu": "接口",
        "部署": "部署", "部shǔ": "部署",
        "架构": "架构", "架gòu": "架构",
        "分布式": "分布式", "分布shì": "分布式",
        "容器": "容器", "容qì": "容器",
        "虚拟化": "虚拟化", "虚nǐ化": "虚拟化",
        "负载均衡": "负载均衡", "负dài均衡": "负载均衡",
        "消息队列": "消息队列", "消xī队列": "消息队列",
        "缓存": "缓存", "缓cún": "缓存",
        "反向代理": "反向代理", "反向代lǐ": "反向代理",
        "容器化": "容器化", "容qì化": "容器化",
        "微内核": "微内核", "围内核": "微内核",
        "灰度发布": "灰度发布", "灰度fā布": "灰度发布",
        "蓝绿部署": "蓝绿部署", "蓝绿部shǔ": "蓝绿部署",
        "服务网格": "服务网格", "服务wǎng格": "服务网格",
        "链路追踪": "链路追踪", "链路追zōng": "链路追踪",
        # ----- 法律 legal -----
        "管辖权": "管辖权", "管xiá权": "管辖权",
        "违约金": "违约金", "违yuē金": "违约金",
        "赔偿金": "赔偿金", "赔cháng金": "赔偿金",
        "管辖": "管辖", "管xiá": "管辖",
        "诉讼": "诉讼", "诉sòng": "诉讼",
        "仲裁": "仲裁", "仲cái": "仲裁",
        "举证": "举证", "举zhèng": "举证",
        "质证": "质证", "质zhèng": "质证",
        "答辩": "答辩", "答biàn": "答辩",
        "调解": "调解", "调jiě": "调解",
        "判决": "判决", "判jué": "判决",
        "裁定": "裁定", "裁dìng": "裁定",
        "立案": "立案", "立àn": "立案",
        "上诉": "上诉", "上sù": "上诉",
        "抗诉": "抗诉", "抗sù": "抗诉",
        "申诉": "申诉", "申sù": "申诉",
        "辩护": "辩护", "辩hù": "辩护",
        "代理": "代理", "代lǐ": "代理",
        # ----- 医学 medical -----
        "诊断": "诊断", "诊duàn": "诊断",
        "处方": "处方", "处fāng": "处方",
        "病理": "病理", "病lǐ": "病理",
        "手术": "手术", "手shù": "手术",
        "麻醉": "麻醉", "麻zuì": "麻醉",
        "注射": "注射", "注shè": "注射",
        "化疗": "化疗", "化liáo": "化疗",
        "放疗": "放疗", "放liáo": "放疗",
        "透析": "透析", "透xī": "透析",
        "造影": "造影", "造yǐng": "造影",
        "活检": "活检", "活jiǎn": "活检",
        "穿刺": "穿刺", "穿cì": "穿刺",
        "处方药": "处方药", "处fāng药": "处方药",
        "抗生素": "抗生素", "抗生sù": "抗生素",
        "高血压": "高血压", "高血yā": "高血压",
        "糖尿病": "糖尿病", "糖尿bìng": "糖尿病",
        "冠心病": "冠心病", "冠心bìng": "冠心病",

        # ----- v0.7.0+: 高频 ASR 误识别 (任何会议都生效的通用 fallback) -----

        # 数字 1-10 (中文口头数字 ASR 经常混)
        "零": "零", "〇": "零",
        "一": "一", "壹": "一",
        "二": "二", "贰": "二",
        "三": "三", "叁": "三",
        "四": "四", "肆": "四",
        "五": "五", "伍": "五",
        "六": "六", "陆": "六",
        "七": "七", "柒": "七",
        "八": "八", "捌": "八",
        "九": "九", "玖": "九",
        "十": "十", "拾": "十",

        # 量词
        "一个": "一个", "壹个": "一个", "一gè": "一个",
        "一种": "一种",
        "一次": "一次", "一cì": "一次",
        "一定": "一定", "一dìng": "一定",
        "一样": "一样", "一yàng": "一样",
        "一直": "一直", "一zhí": "一直",
        "一起": "一起", "一qǐ": "一起",
        "一边": "一边", "一biān": "一边",
        "一下": "一下", "一xià": "一下",

        # 基础会议词 (SenseVoice 经常错)
        "会议": "会议", "会yì": "会议", "惠议": "会议",
        "纪要": "纪要", "纪yào": "纪要", "纪药": "纪要",
        "记录": "记录", "记lù": "记录",
        "讨论": "讨论", "讨lùn": "讨论",
        "决定": "决定", "绝dìng": "决定",
        "结论": "结论", "结lùn": "结论", "解论": "结论",
        "结果": "结果", "结guǒ": "结果",
        "总结": "总结", "总jié": "总结",
        "方案": "方案", "方àn": "方案",
        "计划": "计划", "jì划": "计划",
        "目标": "目标", "木biāo": "目标",
        "任务": "任务", "任wù": "任务",
        "进度": "进度", "尽dù": "进度",
        "反馈": "反馈", "反kuì": "反馈",
        "确认": "确认", "确rèn": "确认",
        "沟通": "沟通", "勾tōng": "沟通",
        "对接": "对接", "对jiē": "对接",
        "协作": "协作", "协zuò": "协作",
        "支持": "支持", "只chí": "支持",
        "配合": "配合", "配hé": "配合",
        "推进": "推进", "推jìn": "推进",
        "完成": "完成", "玩chéng": "完成",
        "结束": "结束", "结shù": "结束",
        "开始": "开始", "开shǐ": "开始",
        "临时": "临时", "临shí": "临时",
        "及时": "及时", "即shí": "及时",
        "尽快": "尽快", "尽kuài": "尽快",
        "差不多": "差不多", "差bu多": "差不多",
        "怎么": "怎么", "zěn么": "怎么", "争么": "怎么",
        "什么": "什么", "shén么": "什么", "神马": "什么",
        "这个": "这个", "这gè": "这个", "遮个": "这个",
        "那个": "那个", "那gè": "那个",
        "我们": "我们", "wǒ们": "我们", "wo们": "我们",
        "你们": "你们", "nǐ们": "你们",
        "他们": "他们", "tā们": "他们",
        "自己": "自己", "zì己": "自己",

        # 时间相关
        "今天": "今天", "金tiān": "今天",
        "明天": "明天", "名tiān": "明天",
        "昨天": "昨天", "昨tiān": "昨天",
        "现在": "现在", "献zài": "现在",
        "马上": "马上", "马shàng": "马上",
        "已经": "已经", "已jīng": "已经",
        "刚才": "刚才", "钢cái": "刚才",
        "晚上": "晚上", "晚shàng": "晚上",
        "早上": "早上", "早shàng": "早上",
        "中午": "中午", "中wǔ": "中午",
        "下午": "下午", "下wǔ": "下午",
        "上周": "上周", "上zhōu": "上周",
        "本周": "本周", "本zhōu": "本周",
        "上次": "上次", "上cì": "上次",
        "下次": "下次", "下cì": "下次",
        "小时": "小时", "小shí": "小时",
        "分钟": "分钟", "分zhōng": "分钟",
        "秒钟": "秒钟", "秒zhōng": "秒钟",
    }
    # v0.7.0+: 始终触发 L1 同音纠错 (不再受 if words 门控).
    # 理由: STATIC_HOMO 表里的 key 都是 ASR 误识别形式 (拼音 + 错字),
    # 真实正常文本中很少出现这些错字串, 误伤风险极低.
    # 关闭门控后, 即便用户没选行业词库 (pack='none'), 通用工程段
    # (前端/后端/数据库/接口 等) 仍能提供保底纠错.
    for k, v in STATIC_HOMO.items():
        if k == v: continue
        out = out.replace(k, v)

    # ----- L2. 动态同音 – 中文 zh-EN keyboard neighbours -----
    # 用户 hotword 是 set; 若用户列表里包含 [金, 吉, 今] 同音候选, 替换 [金]→[吉] 等
    ZH_HOMO_BANK = {
        "金": ["今", "津", "锦"],
        "吉": ["金", "急", "及"],
        "己": ["已", "以"],
        "未": ["末", "味"],
        "司": ["丝", "思"],
        "丽": ["黎", "李"],
        "立": ["力", "利"],
        "六": ["陆", "溜"],
        "鲍": ["抱", "暴"],
    }
    for canon, alts in ZH_HOMO_BANK.items():
        if not any(canon in w for w in words):
            continue
        for alt in alts:
            # 仅替换独立字符/词 (避免 [金条]→[今条])
            out = _re.sub(rf"(?<=[\\u4e00-\\u9fff]){alt}(?=[\\u4e00-\\u9fff])", canon, out)

    # ----- L3. 多长度 fuzzy match (char-level ratio) -----
    # v0.7.0+: 临时禁用 L3 fuzzy 替换. 历史 bug: "离线会议" (真实词, 4字) vs hotword
    # "离线会记" (4字) ratio 0.75 触发替换, 把"会议"误改为"会记". L0/L1 硬规则已
    # 覆盖绝大多数 ASR 误识别场景, L3 误伤率太高, 暂时禁用. 后续可换 jieba + 编辑
    # 距离重写 L3.
    if os.environ.get("MEETILY_L3_FUZZY") == "1":
        try:
            out = _hotword_fuzzy_replace(out, words, threshold)
        except Exception as _e:
            sys.stderr.write(f"[sherpa_asr] hotword fuzzy pass failed: {_e}\n")

    # v0.7.0+: 热词加载日志 (确保 "代码生效" 在 stderr 可见)
    n_words = len(words) if words else 0
    sys.stderr.write(
        f"[sherpa_asr] hotword_bias: words={n_words} l0_hits={l0_hits}\n"
    )

    # 最后: 删末尾标点 (SenseVoice 偶尔出 [好。.] [xx, ,,,])
    out = _re.sub(r"[\s,。.；;、,!?！？\-—_]+$", "", out).strip()
    return out


def _hotword_fuzzy_replace(text: str, words: list, threshold: float) -> str:
    """对每个 hotword 在 text 中找最佳模糊窗口, 命中后替换.

    改进点 vs 原 difflib 版:
      – 优先 long-word (避免短词贪婪匹配吃掉长词前缀)
      – 替换采用 weighted ratio, 长度差异惩罚, 防止一字替换 3 字
      – 跨字块扫: 不只是 [i:i+sl], 而是先按 2-7 字 keyword 长度分级
    """
    import difflib
    # v0.7.0+: 只 fuzzy 匹配 4 字以上 hotword, 短词 (2-3 字) 太容易误命中子串
    sorted_words = sorted([w for w in (w.strip() for w in words) if w and len(w) >= 4],
                          key=len, reverse=True)
    out = text
    used_spans = []  # (start, end) 已修改过的, 避免短词覆盖

    def overlap(a, b):
        return not (a[1] <= b[0] or b[1] <= a[0])

    for w in sorted_words:
        L = len(w)
        best = None
        # v0.7.0+: 只允许相同长度的窗口 (sl == L), 避免 "离线会" 替换 "离线会议" 这种
        # 子串误匹配. ±1 字差异全部禁用, 留给 L0/L1 硬规则处理.
        sl = L
        if sl < 2 or sl > len(out):
            continue
        for i in range(len(out) - sl + 1):
            if any(overlap((i, i+sl), s) for s in used_spans):
                continue
            sub = out[i:i+sl]
            ratio = difflib.SequenceMatcher(None, sub, w).ratio()
            if ratio >= threshold and (best is None or ratio > best[0]):
                best = (ratio, i, sl)
        if best:
            _, idx, sl = best
            out = out[:idx] + w + out[idx+sl:]
            used_spans.append((idx, idx + L))
    return out


def _trim_silence(samples, sr: int, threshold_db: float = -45.0,
                  min_silence_ms: int = 350, frame_ms: int = 30) -> tuple:
    """v0.6.10+: RMS-based silence trim.

    输入 float32 1D ndarray, 输出 (kept_samples, segments)
    segments = [(start_sec, end_sec), ...] 切分后的句级时间轴

    - frame 30ms 滑窗, 每帧 RMS (dB) < threshold 视为静音
    - 连续静音 >= min_silence_ms 切句
    """
    import numpy as np
    if samples is None or len(samples) == 0:
        return samples, []
    frame_len = int(sr * frame_ms / 1000)
    if frame_len < 1: frame_len = 1
    n_frames = len(samples) // frame_len
    if n_frames < 2:
        return samples, [(0.0, len(samples) / sr)]
    # 切帧
    frames = samples[:n_frames * frame_len].reshape(n_frames, frame_len)
    rms = np.sqrt((frames.astype(np.float32) ** 2).mean(axis=1) + 1e-12)
    db = 20.0 * np.log10(rms + 1e-12)
    is_voice = db > threshold_db
    # 连续静音 ≥ min_silence_ms 切句
    min_silence_frames = max(1, int(min_silence_ms / frame_ms))
    segments = []
    seg_start = None
    silence_run = 0
    for i, v in enumerate(is_voice):
        if v:
            if seg_start is None:
                seg_start = i
            silence_run = 0
        else:
            silence_run += 1
            if seg_start is not None and silence_run >= min_silence_frames:
                # 切句: 包含静音前 0.5 帧 (避免吃掉尾音)
                seg_end = max(seg_start + 1, i - max(1, min_silence_frames // 4))
                segments.append((seg_start * frame_len / sr, (seg_end + 1) * frame_len / sr))
                seg_start = None
                silence_run = 0
    if seg_start is not None:
        seg_end = n_frames
        segments.append((seg_start * frame_len / sr, seg_end * frame_len / sr))
    # 若全静音 (无 segments), 整段返
    if not segments:
        return samples, [(0.0, len(samples) / sr)]
    # 重新拼接保留片段
    keep = np.concatenate([samples[int(s * sr):int(e * sr)] for s, e in segments])
    return keep.astype(np.float32), segments


# ----- Streaming pipeline (v0.6.11 增量实时识别) -----
#
# SenseVoice 是 NAR, 严格意义不支持 streaming; 但 Paraformer / 任一 offline 模型都可以做
# "chunked 模拟流式": 每 0.6-1s 对累计 buffer 跑一次 OfflineRecognizer, 输出 delta + final
#
# Session 设计:
#   stream_begin:  为该 session 开 buffer, 记录 model_tag, hotwords 等
#   stream_chunk:  追加音频, 每 ≤1.2s 触发一次 offline decode, 推 partial (delta vs 上次 final state)
#   stream_finalize: 把最后残留 buffer 跑完, 出 final; 清空 session
#
# 静音切句: 用 _trim_silence 的 is_voice mask, 累计 ≥ silence_threshold_ms 静音触发 final+restart buffer

_STREAM_SESSIONS = {}
_DEFAULT_HOTWORDS = ("none", "")


def _stream_session_id(tag: str) -> str:
    import time as _t
    return f"{tag}-{int(_t.time() * 1000)}"


def _stream_session_begin(req):
    """Begin a new streaming session."""
    rid = req.get("id", _stream_session_id(req.get("tag", "?")))
    sessions_tag = req.get("model", "paraformer-zh")
    hw_pack = req.get("hotwords_pack", "none")
    hw_custom = req.get("hotwords_custom", "")
    sample_rate = int(req.get("sample_rate", 16000))
    _STREAM_SESSIONS[rid] = {
        "model": sessions_tag,
        "sample_rate": sample_rate,
        "buffer": [],  # list of float32 chunks
        "final_text": "",  # already emitted
        "silence_run": 0,
        "last_emit_t": 0.0,
        "last_decode_t": 0.0,
        "hw_pack": hw_pack,
        "hw_custom": hw_custom,
        "chunk_threshold_ms": int(req.get("chunk_threshold_ms", 600)),
        "silence_threshold_ms": int(req.get("silence_threshold_ms", 1200)),
        # v0.6.11+: 累计缓冲超过此值, 强制 emit final (解决长会议无静音切句)
        "force_final_ms": int(req.get("force_final_ms", 30000)),
        "last_forced_t": 0.0,
        "created": time.time(),
    }
    return {"id": rid, "ok": True, "action": "stream_begin", "session_id": rid}


def _stream_session_chunk(req):
    """Append audio + (maybe) emit partial / final."""
    rid = req.get("id", "")
    sess = _STREAM_SESSIONS.get(rid)
    if not sess:
        return {"id": rid, "ok": False, "error": "unknown session (call stream_begin first)", "action": "stream_chunk"}

    import base64, numpy as _np
    if "audio_b64" not in req:
        return {"id": rid, "ok": False, "error": "missing audio_b64", "action": "stream_chunk"}
    raw = base64.b64decode(req["audio_b64"])
    arr = _np.frombuffer(raw, dtype=_np.float32)
    if arr.size == 0:
        return {"id": rid, "ok": True, "action": "stream_chunk", "partial": "", "delta": "", "segments_emitted": 0, "is_endpoint": False}

    sr = sess["sample_rate"]
    sess["buffer"].append(arr)
    buf = _np.concatenate(sess["buffer"])

    # RMS-based silence detection
    FRAME_MS = 30
    SILENCE_DB = -45.0
    frame_len = max(1, sr * FRAME_MS // 1000)
    if len(buf) < frame_len * 2:
        return {"id": rid, "ok": True, "action": "stream_chunk", "partial": "", "delta": "", "segments_emitted": 0, "is_endpoint": False}

    # 切尾端若干帧做 RMS
    tail_n = frame_len * 4  # 120ms 末尾
    tail = buf[-tail_n:].reshape(-1, frame_len) if len(buf) >= tail_n else buf.reshape(-1, frame_len)
    rms = _np.sqrt((tail.astype(_np.float32) ** 2).mean(axis=1) + 1e-12)
    db = 20.0 * _np.log10(rms + 1e-12)
    silent_frames = int((db <= SILENCE_DB).sum())
    is_silent_now = silent_frames >= (len(tail) - 2)  # 末尾帧多数静音
    if is_silent_now:
        sess["silence_run"] += FRAME_MS
    else:
        sess["silence_run"] = 0

    # 仅当累计缓冲 ≥ chunk_threshold_ms 才跑 decode
    buf_ms = int(len(buf) / sr * 1000)
    elapsed_ms = (time.time() - sess["last_decode_t"]) * 1000
    emit_partial = buf_ms >= sess["chunk_threshold_ms"] and elapsed_ms >= sess["chunk_threshold_ms"]
    # 静音切 final: 静音 ≥ silence_threshold_ms 且 buffer 已发声
    is_endpoint = (sess["silence_run"] >= sess["silence_threshold_ms"]
                   and buf_ms >= sess["silence_threshold_ms"])
    # v0.6.11+ bug fix: 长会议连续说话 buffer 累积超 force_final_ms 必须强制切句
    force_final = buf_ms >= sess["force_final_ms"] and (time.time() - sess["last_forced_t"]) > 5.0
    if force_final:
        sess["last_forced_t"] = time.time()
    # v0.6.11+: force_final 也报告为 is_endpoint (前端 UI 切段)
    is_endpoint_effective = is_endpoint or force_final

    if not emit_partial and not is_endpoint_effective:
        return {"id": rid, "ok": True, "action": "stream_chunk",
                "partial": "", "delta": "", "segments_emitted": 0,
                "is_endpoint": False, "buffered_ms": buf_ms,
                "force_final": force_final}

    # 跑一次 offline decode
    try:
        rec, loaded_tag = _ensure_model(sess["model"])
    except Exception as e:
        return {"id": rid, "ok": False, "error": f"model load failed: {e}", "action": "stream_chunk"}

    decode_t = time.time()
    s = rec.create_stream()
    s.accept_waveform(sr, buf.astype(_np.float32))
    rec.decode_streams([s])
    decode_ms = int((time.time() - decode_t) * 1000)
    raw_text = (s.result.text or "").strip()
    if loaded_tag.startswith("sensevoice") and raw_text.startswith("[") and "]" in raw_text[:30]:
        raw_text = raw_text.split("]", 1)[-1].strip()
    # 热词 bias
    try:
        words = get_hotwords(sess["hw_pack"], sess["hw_custom"])
        if words:
            raw_text = _apply_hotword_bias(raw_text, words)
    except Exception:
        pass
    # ITN 后处理 (v0.6.13+)
    try:
        raw_text = apply_itn(raw_text)
    except Exception as e:
        sys.stderr.write(f"[sherpa_asr] itn postprocess failed: {e}\n")

    sess["last_decode_t"] = time.time()
    response_segments = 0
    delta_text = ""
    partial_text = ""

    if is_endpoint_effective:
        # 段尾: 把当前整段 raw_text 当 final, 重置 buffer (v0.6.11+ 含 force_final)
        if raw_text:
            partial_text = raw_text
            # 仅追加新增 (按 final_text 前缀校验)
            if raw_text.startswith(sess["final_text"]):
                delta_text = raw_text[len(sess["final_text"]):]
            else:
                # 不一致 (rare), 全追加
                delta_text = raw_text
                sess["final_text"] = raw_text
            if delta_text:
                sess["final_text"] = sess["final_text"] + delta_text
                response_segments = 1
        # 切句: 保留 200ms overlap (防边界丢字)
        keep = max(0, len(buf) - int(0.2 * sr))
        sess["buffer"] = [buf[keep:]] if keep < len(buf) else []
        sess["silence_run"] = 0
        sess["final_text"] = ""  # buffer 已重置, final 重新累计
    else:
        # partial: 仅 emit 末尾 delta
        partial_text = raw_text
        if raw_text.startswith(sess["final_text"]):
            delta_text = raw_text[len(sess["final_text"]):]

    return {
        "id": rid, "ok": True, "action": "stream_chunk",
        "partial": partial_text,
        "delta": delta_text,
        "segments_emitted": response_segments,
        "is_endpoint": is_endpoint_effective,
        "is_endpoint_natural": is_endpoint,  # 静音 vs 强制切句
        "force_final": force_final,
        "buffered_ms": buf_ms,
        "decode_ms": decode_ms,
        "model": loaded_tag,
    }


def _stream_session_finalize(req):
    """v0.6.11+: close session, force-run offline decode on leftover buffer.

    这是"录音结束兜底": 即使 streaming 没触发任何 endpoint, finalize 必跑一次整段识别,
    任何残留 buffer 都能被 emit 成 final delta."""
    rid = req.get("id", "")
    sess = _STREAM_SESSIONS.pop(rid, None)
    if not sess:
        return {"id": rid, "ok": True, "action": "stream_finalize", "segments_emitted": 0}
    import numpy as _np
    if not sess["buffer"]:
        return {"id": rid, "ok": True, "action": "stream_finalize", "segments_emitted": 0}
    buf = _np.concatenate(sess["buffer"]) if len(sess["buffer"]) > 1 else sess["buffer"][0]
    sr = sess["sample_rate"]
    try:
        rec, loaded_tag = _ensure_model(sess["model"])
        s = rec.create_stream()
        s.accept_waveform(sr, buf.astype(_np.float32))
        rec.decode_streams([s])
        raw_text = (s.result.text or "").strip()
        if loaded_tag.startswith("sensevoice") and raw_text.startswith("[") and "]" in raw_text[:30]:
            raw_text = raw_text.split("]", 1)[-1].strip()
        try:
            words = get_hotwords(sess["hw_pack"], sess["hw_custom"])
            if words:
                raw_text = _apply_hotword_bias(raw_text, words)
        except Exception:
            pass
        # ITN 后处理 (v0.6.13+)
        try:
            raw_text = apply_itn(raw_text)
        except Exception as e:
            sys.stderr.write(f"[sherpa_asr] itn postprocess failed: {e}\n")
        if raw_text.startswith(sess["final_text"]):
            delta = raw_text[len(sess["final_text"]):]
        else:
            delta = raw_text
            sess["final_text"] = raw_text
        return {"id": rid, "ok": True, "action": "stream_finalize",
                "delta": delta, "model": loaded_tag, "segments_emitted": 1 if delta else 0}
    except Exception as e:
        return {"id": rid, "ok": False, "error": str(e), "action": "stream_finalize"}



def transcribe(req):
    rid = req.get("id", "")
    tag = req.get("model", "paraformer-zh")
    t0 = time.time()

    arr, sr = _load_audio(req)
    # v0.6.10+: RMS silence trim (-45dB, 350ms 切句)
    try:
        import numpy as _np
        if arr.ndim == 1:
            arr, _segs = _trim_silence(arr, sr)
    except Exception:
        _segs = []
    rec, loaded_tag = _ensure_model(tag)
    load_ms = 0  # already counted inside _ensure_model but track total load
    decode_t = time.time()
    streams = []
    if loaded_tag == "funasr-nano-zh" and len(arr) / sr > 12.0:
        max_samples = int(sr * 12.0)
        min_samples = int(sr * 3.0)
        cursor = 0
        while cursor < len(arr):
            hard_end = min(cursor + max_samples, len(arr))
            end = hard_end
            if hard_end < len(arr):
                search_start = min(cursor + min_samples, hard_end)
                window = arr[search_start:hard_end]
                if len(window):
                    import numpy as _np
                    frame = max(1, int(sr * 0.02))
                    energies = [float(_np.mean(_np.abs(window[i:i + frame]))) for i in range(0, len(window), frame)]
                    if energies:
                        quiet_index = min(range(len(energies)), key=energies.__getitem__)
                        candidate_end = search_start + quiet_index * frame
                        if candidate_end > cursor + min_samples:
                            end = candidate_end
            chunk = arr[cursor:end]
            if len(chunk):
                stream = rec.create_stream()
                stream.accept_waveform(sr, chunk)
                streams.append(stream)
            cursor = end
        sys.stderr.write(f"[sherpa_asr] nano chunking: audio={len(arr)/sr:.2f}s chunks={len(streams)} max=12s\n")
    else:
        stream = rec.create_stream()
        stream.accept_waveform(sr, arr)
        streams.append(stream)
    rec.decode_streams(streams)
    decode_ms = int((time.time() - decode_t) * 1000)
    duration_ms = int((time.time() - t0) * 1000)

    raw_text = "".join(stream.result.text.strip() for stream in streams)
    if not raw_text and len(arr) / sr >= 1.0:
        raise RuntimeError(f"{loaded_tag} returned empty transcript for {len(arr)/sr:.2f}s audio")
    # SenseVoice emotion/lang token cleanup (e.g. "<|zh|><|HAPPY|>你好" -> "你好")
    if loaded_tag.startswith("sensevoice") and raw_text.startswith("[") and "]" in raw_text[:30]:
        raw_text = raw_text.split("]", 1)[-1].strip()
    # 热词后处理 bias (W2.7)
    hw_pack = req.get("hotwords_pack", "none")
    hw_custom = req.get("hotwords_custom", "")
    if hw_pack and hw_pack != "none" or hw_custom:
        words = get_hotwords(hw_pack, hw_custom)
        if words:
            raw_text = _apply_hotword_bias(raw_text, words)
    # ITN 后处理 (v0.6.13+)
    try:
        raw_text = apply_itn(raw_text)
    except Exception as e:
        sys.stderr.write(f"[sherpa_asr] itn postprocess failed: {e}\n")

    resp = {
        "id": rid,
        "ok": True,
        "text": raw_text,
        "confidence": 0.92,
        "decode_ms": decode_ms,
        "duration_ms": duration_ms,
        "model": loaded_tag,
        "audio_seconds": round(len(arr) / sr, 2),
    }

    # v0.6.14+ Speaker Diarization (optional, if model files exist)
    # Only run for >= 10s audio to avoid false positives on very short clips.
    # Async'd behind a thread to not block transcription response.
    # v0.7.0+: 也返回 segments 数组 (speaker + start + end + text), 供前端按人分段展示.
    if diar_is_available() and len(arr) / sr >= 10.0:
        try:
            import threading as _threading
            from diar import process_diarization
            _diar_state = {"result": None, "err": None}
            def _run_diar():
                try:
                    # v0.7.0+ 新 API: 返回完整 result (含 segments), fallback 旧 API
                    _diar_state["result"] = process_diarization(arr, sr)
                except (AttributeError, ImportError):
                    # 旧 diar.py: 只返 num_speakers
                    n = count_speakers(arr, sr)
                    _diar_state["result"] = {"num_speakers": n, "segments": []}
                except Exception as e:
                    _diar_state["err"] = e
            _t = _threading.Thread(target=_run_diar, daemon=True)
            _t.start()
            _t.join(timeout=12.0)  # max 12s, diar 实际 ~1-3s
            if _diar_state["result"]:
                r = _diar_state["result"]
                if r.get("num_speakers") and r.get("num_speakers") > 0:
                    resp["num_speakers"] = int(r["num_speakers"])
                if r.get("num_speakers") and r.get("segments"):
                    # segments 已经是 [{start, end, speaker, duration, text}]
                    resp["segments"] = r["segments"]
                    sys.stderr.write(
                        f"[sherpa_asr] diar: {resp.get('num_speakers', '?')} speakers, "
                        f"{len(r['segments'])} segments\n"
                    )
            elif _diar_state["err"]:
                sys.stderr.write(f"[sherpa_asr] diar error: {_diar_state['err']}\n")
        except Exception as e:
            sys.stderr.write(f"[sherpa_asr] diar dispatch failed: {e}\n")

    # Level 3: 字级 timestamps (Pro 模式 + RAM>=8GB + 模型支持)
    # 仅当客户端显式请求 timestamps=True 时返回, 默认 False 节省 IPC 载荷.
    want_ts = bool(req.get("timestamps", False))
    cap = _capability()
    if want_ts and cap["level3_supported"] and cap["model_timestamp_support"].get(loaded_tag, False):
        tokens = list(s.result.tokens or [])
        timestamps = list(s.result.timestamps or [])
        # SenseVoice token 是字级 (单字), Paraformer 是 id, 这里只对 sensevoice 返回
        if tokens and timestamps and len(tokens) == len(timestamps):
            # 转 timestamps 为秒 (sherpa-onnx 已是秒, 但 type=double, JSON serializable)
            resp["tokens"] = tokens
            resp["timestamps"] = [float(t) for t in timestamps]

    return resp


def handle(req):
    action = req.get("action", "transcribe")
    if action == "list":
        return {
            "id": req.get("id", ""),
            "ok": True,
            "action": "list",
            "models": _list_installed(),
        }
    if action == "ping":
        return {"id": req.get("id", ""), "ok": True, "action": "ping", "models": _list_installed()}
    if action == "capability":
        cap = _capability()
        cap.update({"id": req.get("id", ""), "ok": True, "action": "capability"})
        return cap
    if action == "transcribe":
        try:
            return transcribe(req)
        except Exception as e:
            tb = traceback.format_exc()
            sys.stderr.write(f"[sherpa_asr] transcribe error: {e}\n{tb}\n")
            return {"id": req.get("id", ""), "ok": False, "error": str(e), "trace": tb}
    if action == "stream_begin":
        return _stream_session_begin(req)
    if action == "stream_chunk":
        return _stream_session_chunk(req)
    if action == "stream_finalize":
        return _stream_session_finalize(req)
    return {"id": req.get("id", ""), "ok": False, "error": f"unknown action '{action}'"}


def _detect_total_ram_gb():
    """Return total RAM in GB. macOS via sysctl; Linux via /proc/meminfo; fallback 16."""
    try:
        if sys.platform == "darwin":
            import subprocess
            out = subprocess.check_output(["sysctl", "-n", "hw.memsize"]).decode().strip()
            return int(int(out) / (1024**3))
        else:
            with open("/proc/meminfo") as f:
                for line in f:
                    if line.startswith("MemTotal:"):
                        return int(int(line.split()[1]) / (1024**2))
    except Exception:
        pass
    return 16  # safe default


def _capability():
    """Daemon capability report (consumed by Rust frontend at startup).
    Level 3 字级 timestamps 只在 total RAM >= 8GB 时开启, 否则降级为 VAD 段循环.
    Paraformer-zh INT8 不返回 timestamps (模型本身不支持), SenseVoice-zh INT8 支持.
    """
    total_gb = _detect_total_ram_gb()
    return {
        "level3_supported": total_gb >= 8,
        "streaming_supported": True,  # chunked offline-decode fake streaming
        "total_ram_gb": total_gb,
        "model_timestamp_support": {
            "sensevoice-zh": True,
            "paraformer-zh": False,
            "zipformer-zh": True,
        },
    }


def main():
    sys.stderr.write(f"[sherpa_asr] daemon started, models_root={MODELS_ROOT}\n")
    pre = _list_installed()
    sys.stderr.write(f"[sherpa_asr] discovered {len(pre)} model packs: {[m['tag'] for m in pre]}\n")
    cap = _capability()
    sys.stderr.write(f"[sherpa_asr] capability: level3={cap['level3_supported']} ram={cap['total_ram_gb']}GB\n")
    sys.stderr.flush()

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
            resp = handle(req)
        except Exception as e:
            tb = traceback.format_exc()
            resp = {"ok": False, "error": str(e), "trace": tb}
            sys.stderr.write(f"[sherpa_asr] ERROR {e}\n{tb}\n")
            sys.stderr.flush()
        sys.stdout.write(json.dumps(resp, ensure_ascii=False) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
