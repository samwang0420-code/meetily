"""ITN 后处理 (v0.6.13+) — 离线会记 B 方案第一步

sherpa-onnx use_itn=True 已经做:
  - 中文数字 -> 阿拉伯数字
  - 标点智能补全
  - 英文大小写

本模块聚焦 sherpa 漏掉的 / 后处理的边缘 case:
  1. 英文缩写合并: O K -> OK, U S A -> USA (1-5 字母,大写)
  2. 重复标点合并: 。。。 -> 。
  3. 中英混合标点后空格: "中文。English" -> "中文。 English"
  4. 数字串内部空格删除: "2025 06 18" -> "20250618" / "3 0" -> "30"
  5. 中文 + 数字 之间空格删除
  6. 中文 + 中文标点 之间空格删除: "走 。" -> "走。"
  7. 多空白清理

调用入口: apply_itn(text: str) -> str
"""
import re


# 中文: CJK Unified Ideographs + 全角标点
_CN_CHAR = r"\u4e00-\u9fff"
_CN_PUNCT = r"\u3000-\u303f\uff00-\uffef"
_CN_ANY = _CN_CHAR + _CN_PUNCT


def _step_merge_acronyms(text: str) -> str:
    """O K / U S A → OK / USA。仅合并 2-5 个大写字母,前后不接中文。"""
    out = []
    i = 0
    n = len(text)
    while i < n:
        ch = text[i]
        if "A" <= ch <= "Z":
            j = i + 1
            while j < n and text[j] == " " and j + 1 < n and "A" <= text[j+1] <= "Z":
                j += 2
            seq = text[i:j].replace(" ", "")
            if 2 <= len(seq) <= 5 and len(seq) < j - i:
                prev = text[i-1] if i > 0 else ""
                nxt = text[j] if j < n else ""
                prev_cn = ("\u4e00" <= prev <= "\u9fff") or ("\u3000" <= prev <= "\u303f")
                next_cn = ("\u4e00" <= nxt <= "\u9fff") or ("\u3000" <= nxt <= "\u303f")
                if not prev_cn and not next_cn:
                    out.append(seq)
                    i = j
                    continue
            out.append(ch)
            i += 1
        else:
            out.append(ch)
            i += 1
    return "".join(out)


def _step_repeat_punct(text: str) -> str:
    text = re.sub(r"。{2,}", "。", text)
    text = re.sub(r"!{2,}", "！", text)
    text = re.sub(r"\?{2,}", "？", text)
    text = re.sub(r"\.{3,}", "……", text)
    text = re.sub(r"，{2,}", "，", text)
    return text


def _step_mixed_spacing(text: str) -> str:
    """中文标点 。！？ + ASCII 字母 之间加空格(已有不动)。"""
    text = re.sub(r"([。！？!?])([A-Za-z])", r"\1 \2", text)
    text = re.sub(r"([A-Za-z])([。！？!?])", r"\1 \2", text)
    return text


def _step_join_digits(text: str) -> str:
    """数字串内空格合并: '2025 06 18' -> '20250618', '3 0' -> '30'
    只处理 [\d] [\d]+ 形式,前后是 word boundary"""
    def repl(m):
        digits = re.findall(r"\d+", m.group(0))
        return "".join(digits)
    return re.sub(r"(?:^|(?<=[\W_]))(\d+(?:[ ]\d+)+)(?=$|[\W_])", repl, text)


def _step_collapse_cn_spaces(text: str) -> str:
    """中文场景的孤立空格删除:
    - 中文汉字 + 空白 + 中文汉字 -> 合并
    - 中文汉字 + 空白 + 数字 -> 合并
    - 数字 + 空白 + 中文汉字 -> 合并
    - 中文汉字 + 空白 + 中文标点 -> 合并 (走 。 -> 走。)
    """
    text = re.sub(rf"([{_CN_ANY}])\s+([{_CN_ANY}])", r"\1\2", text)
    text = re.sub(rf"([{_CN_CHAR}])\s+(\d)", r"\1\2", text)
    text = re.sub(r"(\d)\s+([" + _CN_CHAR + r"])", r"\1\2", text)
    return text


def _step_collapse_spaces(text: str) -> str:
    text = re.sub(r" {2,}", " ", text)
    return text.strip()


def apply_itn(text: str) -> str:
    """主入口: raw sherpa output -> clean"""
    if not text:
        return text
    text = text.strip()
    # 顺序:
    # 1) 重复标点
    text = _step_repeat_punct(text)
    # 2) 基础空白合并
    text = _step_collapse_spaces(text)
    # 3) 数字串空格合并
    text = _step_join_digits(text)
    # 4) 中文场景孤立空格清理
    text = _step_collapse_cn_spaces(text)
    # 5) 中英混合加空格
    text = _step_mixed_spacing(text)
    # 6) 英文缩写合并
    text = _step_merge_acronyms(text)
    return text
