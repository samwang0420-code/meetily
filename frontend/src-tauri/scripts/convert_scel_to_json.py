"""
言镜 AI §111: 搜狗拼音细胞词库 (.scel) → 热词 JSON 转换工具

来源: 用户从搜狗输入法官方/第三方下载的 .scel 细胞词库
许可: 搜狗输入法细胞词库本身属于"用户分享", 无明确开源协议
       本工具仅做格式转换 + 质量过滤, 不重新分发
       转换后的 JSON 跟随言镜 AI 产品一起 ship

输入: 一个或多个 .scel 文件
输出: hotwords_data/{pack_name}.json 格式 (同 §91 已有 6 个 pack 格式)

过滤规则 (防止低质量词污染 ASR 热词):
  - 长度 2-12 字
  - 至少 1 个汉字
  - 排除元数据污染词 (含"方推荐"/"网友上传"/"来源于"等)
  - 排除纯标点 / 纯空白
  - 同词保留最高频

用法:
  python3 convert_scel_to_json.py \\
    --input-dir /Users/wangwei/Documents/离线会记/热词 \\
    --output-dir /Users/wangwei/Documents/离线会记/frontend/src-tauri/scripts/hotwords_data \\
    --max-words 800
"""
import argparse
import json
import os
import struct
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# 搜狗拼音 .scel 格式已知是 UTF-16LE 编码中文字符 + 4 bytes 词频
# 实测 parser: 跳过前 0x200 字节 metadata, 后续按 UTF-16LE 连续中文提取

SKIP_HEADER = 0x200  # 跳过 .scel 前 512 字节 metadata
MIN_WORD_LEN = 3
MAX_WORD_LEN = 10
MIN_FREQ = 0

# GBK→UTF-16LE 误解码的常见乱码字符 (排除)
GARBAGE_CHARS = set('牎蝟晥猶籴喆晔甪叚堃珺珉煊焱晗翀耒臾茀藁叵')

# 元数据污染黑名单 (出现在 .scel header 描述/分类/版权信息)
METADATA_BLACKLIST = [
    '方推荐', '官方推荐', '网友上传', '来源于', '感谢', '下载', '词库',
    '大全', '通用', '基础医学', '农医类', '社科类', '自己上传',
    '感谢网友', '互联网', '由网友', '请勿', '商业', '请联系',
    '保留所有', '权利', '本人', '上传', '欢迎', '使用', '本人对',
    '本词库', '以下', '声明', 'copyright', '版本', '整理',
    '收录', '内容', '如有', '版权', '联系', '本人',
]


def is_valid_word(s: str) -> bool:
    """检查词是否合格"""
    if not s or len(s) < MIN_WORD_LEN or len(s) > MAX_WORD_LEN:
        return False
    # 至少 2 个汉字
    cn_count = sum(1 for c in s if '\u4e00' <= c <= '\u9fff')
    if cn_count < 2:
        return False
    # 排除 GBK 误解码乱码
    if any(c in GARBAGE_CHARS for c in s):
        return False
    # 排除元数据污染
    for bad in METADATA_BLACKLIST:
        if bad in s:
            return False
    # 排除纯标点
    if all(not c.isalnum() and not ('\u4e00' <= c <= '\u9fff') for c in s):
        return False
    return True


def parse_scel(scel_path: str) -> List[Tuple[str, int]]:
    """解析 .scel 文件, 返回 [(word, freq), ...]"""
    with open(scel_path, 'rb') as f:
        data = f.read()
    if len(data) < SKIP_HEADER + 100:
        return []

    pos = SKIP_HEADER
    seen = set()
    words = []

    while pos < len(data) - 100:
        chars = []
        while pos < len(data) - 1:
            cu = data[pos:pos + 2]
            try:
                ch = cu.decode('utf-16-le')
                if ('\u4e00' <= ch <= '\u9fff'
                        or ch in '()（）【】《》、。，；：？！·-'
                        or ch.isdigit()):
                    chars.append(ch)
                    pos += 2
                else:
                    break
            except UnicodeDecodeError:
                break

        if len(chars) < MIN_WORD_LEN:
            pos += 2
            continue

        word = ''.join(chars).strip('()（）【】《》、。，；：？！·-')
        if not is_valid_word(word):
            pos += 2
            continue

        # 跳过 0x00 分隔符
        while pos < len(data) and data[pos] == 0:
            pos += 1

        # 读 4 bytes 词频
        if pos + 4 > len(data):
            break
        try:
            freq = struct.unpack('<I', data[pos:pos + 4])[0]
        except struct.error:
            break
        pos += 4

        # 跳过 0x00
        while pos < len(data) and data[pos] == 0:
            pos += 1

        if freq > MIN_FREQ and word not in seen:
            seen.add(word)
            words.append((word, freq))

    return words


def merge_scel_files(scel_files: List[str]) -> Dict[str, int]:
    """合并多个 .scel, 同词保留最高频"""
    merged: Dict[str, int] = {}
    for f in scel_files:
        words = parse_scel(f)
        for w, freq in words:
            if w in merged:
                merged[w] = max(merged[w], freq)
            else:
                merged[w] = freq
    return merged


def select_top_words(merged: Dict[str, int], max_words: int) -> List[str]:
    """按 freq 降序取前 max_words 个"""
    sorted_words = sorted(merged.items(), key=lambda x: -x[1])
    return [w for w, _ in sorted_words[:max_words]]


def write_json(pack_name: str, words: List[str], source_files: List[str],
               out_dir: str, schema_version: int = 1) -> str:
    """写 hotwords_data/{pack_name}.json"""
    out_path = os.path.join(out_dir, f"{pack_name}.json")
    os.makedirs(out_dir, exist_ok=True)
    data = {
        "schema_version": schema_version,
        "name": pack_name,
        "source": "搜狗拼音细胞词库 (.scel) 转换 + 质量过滤 (用户 2026-08-12 提供)",
        "source_files": source_files,
        "license": "用户分享 (User-shared, 转换后随产品 ship, 不重新分发源 .scel)",
        "raw_word_count": None,  # 由调用者填
        "filtered_count": len(words),
        "min_freq": None,
        "words": words,
    }
    with open(out_path, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    return out_path


# 预定义 pack 分类 (按主题)
PACK_GROUPS = {
    "sogou_medical_curated": [
        "ICD-10疾病编码1.scel",
        "医学词汇大全【官方推荐】.scel",
        "中外药品名称大全【官方推荐】.scel",
        "中医中药大全【官方推荐】.scel",
        "人体解剖学名词【官方推荐】.scel",
        "药企和国外药企.scel",
        "医疗器械大全【官方推荐】.scel",
        "人体穴位名称【官方推荐】.scel",
        "中药药材.scel",
    ],
    "sogou_legal_curated": [
        "法律词汇大全【官方推荐】.scel",
        "法律术语辞典.scel",
        "专利审查术语.scel",
        "法律专业词库整理.scel",
        "刑事诉讼词库.scel",
        "法律文书法规法条.scel",
    ],
}


def main():
    ap = argparse.ArgumentParser(description="搜狗 .scel → 热词 JSON 转换")
    ap.add_argument("--input-dir", required=True, help="含 .scel 文件的目录")
    ap.add_argument("--output-dir", required=True, help="输出 hotwords_data/ 目录")
    ap.add_argument("--max-words", type=int, default=800, help="每个 pack 保留最大词数 (默认 800)")
    ap.add_argument("--pack", choices=list(PACK_GROUPS.keys()) + ["all"], default="all",
                    help="要生成的 pack (默认 all)")
    args = ap.parse_args()

    if not os.path.isdir(args.input_dir):
        print(f"ERROR: input dir not found: {args.input_dir}")
        sys.exit(1)

    # 找 .scel 文件 (去重 md5)
    all_scel = {}
    for fname in os.listdir(args.input_dir):
        if not fname.endswith('.scel'):
            continue
        path = os.path.join(args.input_dir, fname)
        with open(path, 'rb') as f:
            content = f.read()
        import hashlib
        digest = hashlib.md5(content).hexdigest()
        all_scel[digest] = path  # md5 → path, 重复自动去重

    print(f"Input: {len(all_scel)} unique .scel files (去重后)")
    print(f"Output dir: {args.output_dir}")
    print(f"Max words per pack: {args.max_words}")
    print()

    # 决定要生成的 pack
    if args.pack == "all":
        packs = PACK_GROUPS
    else:
        packs = {args.pack: PACK_GROUPS[args.pack]}

    for pack_name, scel_names in packs.items():
        # 找匹配的文件
        matched = []
        for scel_name in scel_names:
            full = os.path.join(args.input_dir, scel_name)
            if os.path.exists(full):
                matched.append(full)
            else:
                # 尝试去重后路径
                with open(full if os.path.exists(full) else os.path.join(args.input_dir, scel_name), 'rb') as f:
                    pass  # skip

        # 用 all_scel dict (md5 → path) 找包含指定文件名的所有去重实例
        # 简化: 直接拿原始 .scel 文件, parse 时会自动去重
        # 这里取所有去重后的 .scel, 但每个 pack 只处理匹配的子集
        # 实际做法: 对每个 pack 名字找对应文件, parse 后 merge
        pack_files = []
        seen_md5 = set()
        for scel_name in scel_names:
            full = os.path.join(args.input_dir, scel_name)
            if not os.path.exists(full):
                continue
            with open(full, 'rb') as f:
                content = f.read()
            digest = hashlib.md5(content).hexdigest()
            if digest not in seen_md5:
                seen_md5.add(digest)
                pack_files.append(full)

        if not pack_files:
            print(f"[SKIP] {pack_name}: no .scel files matched")
            continue

        print(f"[{pack_name}] Parsing {len(pack_files)} files...")
        merged = merge_scel_files(pack_files)
        print(f"  merged unique words: {len(merged)}")

        # 取 top max_words
        top_words = select_top_words(merged, args.max_words)

        # 写 JSON
        out_path = write_json(pack_name, top_words, pack_files, args.output_dir)
        # 补充 raw_word_count
        with open(out_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        data['raw_word_count'] = len(merged)
        with open(out_path, 'w', encoding='utf-8') as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"  [OK] {len(top_words)} words → {out_path}")
        if top_words:
            print(f"  samples: {top_words[:5]}")
        print()

    # 同步 packs_index.json
    index_path = os.path.join(args.output_dir, "packs_index.json")
    if os.path.exists(index_path):
        with open(index_path, 'r', encoding='utf-8') as f:
            index = json.load(f)
    else:
        index = {"schema_version": 1, "packs": {}}
    for pack_name in packs:
        if pack_name in PACK_GROUPS:
            index['packs'][pack_name] = f"{pack_name}.json"
    with open(index_path, 'w', encoding='utf-8') as f:
        json.dump(index, f, ensure_ascii=False, indent=2)
    print(f"[OK] Updated packs_index.json")


if __name__ == "__main__":
    main()
