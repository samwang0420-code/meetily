#!/usr/bin/env python3
"""
v3: 排除 console.* (开发者日志), 只看用户可见 UI
"""
import re, glob
from pathlib import Path

ROOT = Path('/Users/wangwei/Documents/meetily/frontend/src')

EN_HINT = re.compile(
    r'\b(?:the|and|for|with|this|that|are|was|but|not|'
    r'loading|failed|success|error|complete|cancel|saving|'
    r'importing|exporting|opening|closing|connecting|'
    r'settings|recording|transcript|summary|meeting|audio|'
    r'click|tap|drag|select|choose|copy|save|delete|edit|update|'
    r'required|missing|found|enable|disable|start|stop|'
    r'please|try|again|close|continue|next|back|finish|done|'
    r'generate|regenerate|refresh|reload|retry|discard|'
    r'preview|search|filter|sort|view|hide|show|reset|'
    r'open|fold|folders|sub|'
    r'no|any|all|each|first|last|next|previous|'
    r'models|model|provider|providers|key|keys|api|'
    r'open\s|click\s|tap\s|drag\s|'
    r'installed|uninstalled|downloaded|updated|deleted)\b',
    re.IGNORECASE,
)

def is_real_ui_english(text):
    t = text.strip()
    if len(t) < 6:
        return False
    if t.startswith('$') or t.startswith('//'):
        return False
    if '${' in t and re.sub(r'\$\{[^}]+\}', '', t).strip() == '':
        return False  # pure template
    words = re.findall(r'\b[a-zA-Z]+\b', t)
    if len(words) < 2:
        return False
    en_count = sum(1 for w in words if EN_HINT.match(w))
    return en_count >= 1

def scan(fp):
    out = []
    try:
        content = open(fp).read()
        lines = content.split('\n')
        for i, line in enumerate(lines, 1):
            stripped = line.lstrip()
            if stripped.startswith('//') or stripped.startswith('*') or stripped.startswith('/*'):
                continue
            if stripped.startswith('import '):
                continue
            # 跳过所有 console.* (开发者日志)
            if 'console.' in line:
                continue
            # 跳过所有含 /* debug-style 表情包装 的 err log (难以批量改)
            # JSX text
            for m in re.finditer(r'>\s*([A-Za-z][A-Za-z0-9 \-/&.,!?\'":]{6,}?)\s*[<{]', line):
                t = m.group(1).strip()
                if t.startswith('{'):
                    continue
                if '{' in t:
                    static = re.sub(r'\{[^}]+\}', '', t).strip()
                    if not static or len(static) < 6:
                        continue
                    t = static
                if is_real_ui_english(t):
                    out.append((i, t, 'JSX'))
            # attr alt/placeholder/aria-label
            for attr in ('alt','placeholder','aria-label'):
                for m in re.finditer(rf'\b{attr}\s*=\s*"([^"]+)"', line):
                    t = m.group(1).strip()
                    if is_real_ui_english(t):
                        out.append((i, t, attr))
            # toast/alert 字面
            for m in re.finditer(
                r"(?:toast|safeToast|alert)\.\w+\(\s*[`'\"]([^`'\"]+)[`'\"]", line
            ):
                t = m.group(1).strip()
                if is_real_ui_english(t):
                    out.append((i, t, 'toast'))
    except Exception:
        pass
    return out

files = glob.glob(str(ROOT / '**' / '*.tsx'), recursive=True)
files = [f for f in files if 'i18n/locales' not in f and '.test.' not in f]

grand = []
for f in files:
    rel = f.replace(str(ROOT)+'/', '')
    hits = scan(f)
    if hits:
        grand.append((rel, hits))
grand.sort(key=lambda x: -len(x[1]))

total = sum(len(h) for _, h in grand)
print(f"\n========== 用户可见 hard-coded English: {total} hits / {len(grand)} files ==========\n")
for rel, hits in grand[:30]:
    print(f"\n## {rel} ({len(hits)})")
    seen = set()
    for ln, txt, kind in hits:
        if txt in seen:
            continue
        seen.add(txt)
        if len(seen) > 6:
            break
        d = txt[:90] + ('...' if len(txt) > 90 else '')
        print(f"  L{ln} [{kind}]: {d!r}")
    if len(hits) > 6:
        print(f"  ...")
