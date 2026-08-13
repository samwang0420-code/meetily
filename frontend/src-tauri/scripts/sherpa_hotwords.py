"""
言镜 AI v0.8.6+: 热词词库 (开源精选 + 内置行业 + 用户自定义)

开源词库来源:
- THUOCL (清华大学开源中文词库, Apache-2.0): https://github.com/thunlp/THUOCL
  - THUOCL_IT.txt 1.6万 → 精选 300 词 (DF>=5000)
  - THUOCL_law.txt 9896 → 精选 257 词 (DF>=50000)
  - THUOCL_medical.txt 18749 → 精选 249 词 (DF>=10000)
  - THUOCL_caijing.txt 3830 → 精选 176 词
- LaWGPT legal_vocab.txt (MIT): https://github.com/pengxiao-song/LaWGPT
  - 公开核心精选 538 词 (THUOCL_law + LaWGPT + LexPredict 合并去重)
- OMAHA 七巧板医学知识图谱 (CC-BY-4.0): https://github.com/OMAHA/Clinical-Coding
  - 公开核心精选 488 词 (THUOCL_medical + OMAHA 合并去重)

所有词库预打包为 JSON, 跟产品一起 ship, 不依赖运行时网络下载.
词库文件: scripts/hotwords_data/{pack_name}.json

pack 命名约定:
  none: 不启用
  general: THUOCL IT 通用工程 (300 词)
  legal: LaWGPT 法律精选 (538 词)
  medical: OMAHA 医疗精选 (488 词)
  finance: THUOCL 财经 (176 词)
  sogou_legal: 搜狗细胞词库法律精选 (800 词, §111 用户提供 8 个 .scel 转换)
  sogou_medical: 搜狗细胞词库医学精选 (800 词, §111 用户提供 9 个 .scel 转换)

用户 2026-08-12 提供 22 个搜狗拼音 .scel 细胞词库 (法律 8 + 医学 14, 去重后 16 个),
源 .scel 文件不随产品 ship (避免分发问题), 仅 ship 转换 + 质量过滤后的精选 JSON.
"""
import json
import os
import re
from typing import Dict, List, Optional

# 词库 JSON 路径
_HOTWORDS_DATA_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "hotwords_data")

# 6 个内置 pack (name -> file)
PACK_FILES = {
    "general": "thuocl_it.json",
    "legal": "lawgpt_legal_vocab.json",
    "medical": "omaha_medical.json",
    "finance": "thuocl_caijing.json",
    # §111: 搜狗拼音细胞词库精选 (用户 2026-08-12 提供 22 个 .scel, 转换 + 质量过滤)
    "sogou_legal": "sogou_legal_curated.json",
    "sogou_medical": "sogou_medical_curated.json",
    # legacy 内置 (保持向后兼容, 不动 sherpa_hotwords API)
    "legacy_legal": "thuocl_law.json",
    "legacy_medical": "thuocl_medical.json",
}

# 缓存 (避免每次都读 JSON)
_PACK_CACHE: Dict[str, List[str]] = {}


def _load_pack_from_json(pack: str) -> Optional[List[str]]:
    """加载指定 pack 的 JSON 词库, 缓存到 _PACK_CACHE"""
    if pack in _PACK_CACHE:
        return _PACK_CACHE[pack]
    file_name = PACK_FILES.get(pack)
    if not file_name:
        return None
    path = os.path.join(_HOTWORDS_DATA_DIR, file_name)
    if not os.path.exists(path):
        return None
    try:
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
        words = data.get("words", [])
        _PACK_CACHE[pack] = words
        return words
    except (OSError, json.JSONDecodeError) as e:
        import sys
        sys.stderr.write(f"[sherpa_hotwords] failed to load {pack} from {path}: {e}\n")
        return None


# 兼容旧 hardcoded legal/medical (sherpa_hotwords.py 之前内置的 80-100 词精简版)
# 这些是 §40 v0.8.4 实装, 保留作为 fallback, 永不删除
LEGACY_BUILTIN_HOTWORDS = {
    "legal_legacy": [
        "原告", "被告", "第三人", "代理人", "诉讼代理人", "法定代表人", "委托代理人",
        "请求", "诉请", "答辩", "反诉", "管辖", "管辖权", "移送管辖", "指定管辖",
        "举证", "质证", "认证", "证据保全", "鉴定", "公证", "证人", "证言",
        "违约金", "定金", "订金", "保证金", "押金", "赔偿金", "补偿金", "利息",
        "本金", "债权", "债务", "债权人", "债务人", "抵押权", "质权", "留置权",
        "担保", "保证", "连带责任", "一般保证", "抗辩", "同时履行抗辩权", "不安抗辩权",
        "违约责任", "侵权责任", "缔约过失责任", "合同解除", "合同无效", "合同撤销",
        "租赁合同", "买卖合同", "借款合同", "担保合同", "委托合同", "服务合同",
        "知识产权", "专利", "商标", "著作权", "商业秘密", "不正当竞争", "垄断",
        "一审", "二审", "再审", "终审", "上诉", "申诉", "抗诉", "申请执行",
        "强制执行", "执行和解", "财产保全", "行为保全", "先予执行",
        "仲裁", "仲裁协议", "仲裁机构", "仲裁员", "仲裁裁决", "司法解释",
        "开庭", "庭前调解", "调解书", "判决书", "裁定书", "决定书",
        "律师事务所", "律师函", "法律意见书", "尽职调查", "合同审查", "合规",
    ],
    "medical_legacy": [
        "主诉", "现病史", "既往史", "个人史", "家族史", "体格检查", "专科检查",
        "初步诊断", "鉴别诊断", "临床诊断", "修正诊断", "出院诊断",
        "主诊医师", "主治医师", "住院医师", "主任医师", "副主任医师",
        "医嘱", "长期医嘱", "临时医嘱", "处方", "电子病历",
        "血常规", "尿常规", "便常规", "生化全套", "凝血功能", "感染四项",
        "胸片", "CT", "核磁共振", "MRI", "B超", "彩超", "心电图", "动态心电图",
        "胃镜", "肠镜", "支气管镜", "膀胱镜",
        "病理", "活检", "穿刺", "切片",
        "手术", "微创手术", "介入治疗", "保守治疗", "对症治疗", "支持治疗",
        "高血压", "糖尿病", "冠心病", "心肌梗塞", "脑梗塞", "脑出血",
        "肺炎", "哮喘", "慢性阻塞性肺疾病", "肺结节",
        "肝炎", "肝硬化", "脂肪肝", "胆囊炎", "胆结石", "胰腺炎",
        "胃炎", "胃溃疡", "胃癌", "结肠癌", "直肠癌",
        "阿司匹林", "他汀类", "二甲双胍", "胰岛素", "降压药", "抗生素",
        "头孢", "青霉素", "阿莫西林", "左氧氟沙星",
        "门诊", "急诊", "住院部", "ICU", "CCU", "手术室", "麻醉科",
    ],
}


def get_hotwords(pack: str, custom: str = "") -> List[str]:
    """
    pack: 'none' | 'general' | 'legal' | 'medical' | 'finance' | 'legacy_legal' | 'legacy_medical'
    custom: 用户自定义热词 (逗号或换行分隔)
    返回: 完整热词列表 (含 builtin + custom 去重)
    """
    out: List[str] = []

    # 1) 加载主 pack
    if pack and pack != "none":
        # legacy fallback (hardcoded, 永远可工作)
        if pack == "legal_legacy":
            out.extend(LEGACY_BUILTIN_HOTWORDS["legal_legacy"])
        elif pack == "medical_legacy":
            out.extend(LEGACY_BUILTIN_HOTWORDS["medical_legacy"])
        else:
            # JSON pack
            words = _load_pack_from_json(pack)
            if words:
                out.extend(words)

    # 2) 自定义词
    if custom:
        for w in re.split(r"[,;\n\s]+", custom):
            w = w.strip()
            if w and w not in out:
                out.append(w)
    return out


def list_available_packs() -> List[Dict[str, str]]:
    """列出所有可用 pack (含元信息, UI 用)"""
    out = []
    for pack, file_name in PACK_FILES.items():
        path = os.path.join(_HOTWORDS_DATA_DIR, file_name)
        if not os.path.exists(path):
            continue
        try:
            with open(path, "r", encoding="utf-8") as f:
                data = json.load(f)
            out.append({
                "id": pack,
                "name": data.get("name", pack),
                "source": data.get("source", ""),
                "license": data.get("license", ""),
                "word_count": data.get("filtered_count", len(data.get("words", []))),
            })
        except (OSError, json.JSONDecodeError):
            continue
    return out


# 单元测试
if __name__ == "__main__":
    import sys
    print("=== Available packs ===")
    for pack in list_available_packs():
        print(f"  [{pack['id']}] {pack['name']} ({pack['word_count']} words, {pack['license']})")

    print("\n=== get_hotwords('legal') ===")
    legal = get_hotwords("legal", "王伟, 言镜AI")
    print(f"  count={len(legal)}, samples: {legal[:5]} ... {legal[-5:]}")

    print("\n=== get_hotwords('medical') ===")
    med = get_hotwords("medical", "")
    print(f"  count={len(med)}, samples: {med[:5]} ... {med[-5:]}")

    print("\n=== get_hotwords('general') ===")
    gen = get_hotwords("general", "")
    print(f"  count={len(gen)}, samples: {gen[:5]} ... {gen[-5:]}")

    print("\n=== get_hotwords('none', '自定义词1 自定义词2') ===")
    none_pack = get_hotwords("none", "自定义词1 自定义词2")
    print(f"  count={len(none_pack)}, samples: {none_pack}")
