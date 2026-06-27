#!/usr/bin/env python3
"""生成电源助手应用图标 — 简洁的电池+闪电造型，Adwaita 蓝。"""
from PIL import Image, ImageDraw
import os

OUT = os.path.join(os.path.dirname(__file__), "src-tauri", "icons")
os.makedirs(OUT, exist_ok=True)

ACCENT = (53, 132, 228, 255)   # Adwaita 蓝
ACCENT_D = (28, 113, 216, 255)
WHITE = (255, 255, 255, 255)
BG = (255, 255, 255, 0)        # 透明


def make_icon(size):
    # 用 4x 超采样获得平滑边缘
    s = size * 4
    img = Image.new("RGBA", (s, s), BG)
    d = ImageDraw.Draw(img)

    # 圆角方形底（蓝色）
    pad = s * 0.10
    radius = s * 0.22
    d.rounded_rectangle([pad, pad, s - pad, s - pad], radius=radius, fill=ACCENT)

    # 电池主体（白色描边圆角矩形）
    bw = s * 0.40   # 电池宽
    bh = s * 0.52   # 电池高
    bx = (s - bw) / 2
    by = (s - bh) / 2 + s * 0.02
    stroke = max(2, int(s * 0.035))
    d.rounded_rectangle([bx, by, bx + bw, by + bh], radius=s * 0.06,
                        outline=WHITE, width=stroke)

    # 电池正极小帽
    cap_w = bw * 0.34
    cap_h = s * 0.05
    cx = (s - cap_w) / 2
    d.rounded_rectangle([cx, by - cap_h, cx + cap_w, by + 1],
                        radius=cap_h / 2, fill=WHITE)

    # 中间闪电（白色多边形）
    cxm = s / 2
    cym = by + bh / 2
    w = bw * 0.5
    h = bh * 0.62
    bolt = [
        (cxm + w * 0.12, cym - h * 0.5),
        (cxm - w * 0.45, cym + h * 0.10),
        (cxm - w * 0.02, cym + h * 0.10),
        (cxm - w * 0.12, cym + h * 0.5),
        (cxm + w * 0.45, cym - h * 0.10),
        (cxm + w * 0.02, cym - h * 0.10),
    ]
    d.polygon(bolt, fill=WHITE)

    return img.resize((size, size), Image.LANCZOS)


# Tauri 所需的标准图标集
sizes = {
    "32x32.png": 32,
    "128x128.png": 128,
    "128x128@2x.png": 256,
    "icon.png": 512,
}
for name, sz in sizes.items():
    make_icon(sz).save(os.path.join(OUT, name))
    print(f"  生成 {name} ({sz}x{sz})")

print("图标生成完成 ->", OUT)
