#!/usr/bin/env python3
"""
Tauri Icon Generator - Strict Directory Compliance
✅ 100% 匹配您提供的树状结构 (含 @2x / Square* / StoreLogo 命名)
✅ 从内嵌 SVG 渲染 (cairosvg 优先 | PIL 兜底)
✅ 生成 icon.ico (Windows) + icon.icns (macOS)
✅ 透明背景 + 小尺寸智能优化
"""

import sys
from io import BytesIO
from pathlib import Path
from typing import List, Tuple

try:
    from PIL import Image, ImageDraw
except ImportError:
    print("❌ 请安装 Pillow: pip install Pillow")
    sys.exit(1)

# ==================== CONFIG ====================
OUTPUT_DIR = Path("src-tauri/icons")
APP_NAME = "touchpad-mobile"
ANDROID_OUTPUT_DIR = Path("src-tauri/gen/android/app/src/main/res")

# 严格按您提供的树状结构定义 (尺寸, 文件名)
ICON_SPECS: List[Tuple[int, str]] = [
    (32, "32x32.png"),
    (128, "128x128.png"),
    (256, "128x128@2x.png"),  # @2x = 256px
    (30, "Square30x30Logo.png"),
    (44, "Square44x44Logo.png"),
    (71, "Square71x71Logo.png"),
    (89, "Square89x89Logo.png"),
    (107, "Square107x107Logo.png"),
    (142, "Square142x142Logo.png"),
    (150, "Square150x150Logo.png"),
    (284, "Square284x284Logo.png"),
    (310, "Square310x310Logo.png"),
    (50, "StoreLogo.png"),  # Windows Store 要求 50x50
    (256, "icon.png"),  # Tauri 主图标
]

# ICO 所需尺寸 (Windows)
ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]
# ICNS 所需尺寸 (macOS)
ICNS_SIZES = [16, 32, 64, 128, 256, 512, 1024]

# Android 图标尺寸 (density -> size)
ANDROID_SIZES = {
    "mdpi": 48,
    "hdpi": 72,
    "xhdpi": 96,
    "xxhdpi": 144,
    "xxxhdpi": 192,
}

# 优化 SVG (无滤镜依赖，cairosvg/PIL 均可渲染)
SVG_CONTENT = """<svg width="1024" height="1024" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
  <rect x="162" y="262" width="700" height="500" rx="60" fill="#2D2D2D" stroke="#464646" stroke-width="6"/>
  <circle cx="680" cy="580" r="18" fill="#4A90E2" opacity="0.65"/>
  <circle cx="740" cy="630" r="14" fill="#4A90E2" opacity="0.45"/>
  <circle cx="790" cy="675" r="10" fill="#4A90E2" opacity="0.3"/>
  <circle cx="512" cy="512" r="100" fill="#4A90E2" opacity="0.25"/>
  <circle cx="512" cy="512" r="85" fill="#4A90E2" opacity="0.92"/>
  <circle cx="495" cy="495" r="25" fill="white" opacity="0.18"/>
</svg>"""

# 颜色常量 (PIL fallback 用)
COLOR_TRACKPAD = (45, 45, 45, 255)
COLOR_STROKE = (70, 70, 70, 255)
COLOR_TOUCH = (74, 144, 226, 235)
COLOR_TOUCH_GLOW = (74, 144, 226, 65)
COLOR_HIGHLIGHT = (255, 255, 255, 45)
# ===============================================


def render_with_cairosvg(size: int) -> Image.Image:
    """使用 cairosvg 渲染 SVG 到指定尺寸 (高质量)"""
    try:
        import cairosvg

        png_data = cairosvg.svg2png(
            bytestring=SVG_CONTENT.encode("utf-8"),
            output_width=size,
            output_height=size,
        )
        return Image.open(BytesIO(png_data)).convert("RGBA")
    except ImportError:
        raise
    except Exception as e:
        raise RuntimeError(f"cairosvg 渲染失败: {e}")


def render_with_pil(size: int) -> Image.Image:
    """PIL 纯绘制兜底方案 (无外部依赖)"""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    scale = size / 1024.0

    # 小尺寸优化: <48px 简化细节
    simplify = size < 48

    # 触控板主体
    pad_w, pad_h = int(700 * scale), int(500 * scale)
    pad_x, pad_y = int(162 * scale), int(262 * scale)
    corner = max(1, int(60 * scale))
    stroke_w = max(1, int(6 * scale))

    draw.rounded_rectangle(
        [pad_x, pad_y, pad_x + pad_w, pad_y + pad_h],
        radius=corner,
        fill=COLOR_TRACKPAD,
        outline=COLOR_STROKE,
        width=stroke_w,
    )

    if not simplify:
        # 滑动轨迹点
        trajectory = [(680, 580, 18, 0.65), (740, 630, 14, 0.45), (790, 675, 10, 0.3)]
        for cx, cy, r, op in trajectory:
            x, y, rad = int(cx * scale), int(cy * scale), int(r * scale)
            if rad >= 1:
                draw.ellipse(
                    [x - rad, y - rad, x + rad, y + rad],
                    fill=(
                        COLOR_TOUCH[0],
                        COLOR_TOUCH[1],
                        COLOR_TOUCH[2],
                        int(255 * op),
                    ),
                )

    # 触摸点 (双层)
    cx, cy = size // 2, size // 2
    outer_r, inner_r = int(100 * scale), int(85 * scale)
    if outer_r > 0:
        draw.ellipse(
            [cx - outer_r, cy - outer_r, cx + outer_r, cy + outer_r],
            fill=COLOR_TOUCH_GLOW,
        )
    if inner_r > 0:
        draw.ellipse(
            [cx - inner_r, cy - inner_r, cx + inner_r, cy + inner_r], fill=COLOR_TOUCH
        )

    if not simplify and size >= 64:
        hx, hy = int(495 * scale), int(495 * scale)
        h_rad = int(25 * scale)
        if h_rad > 0:
            draw.ellipse(
                [hx - h_rad, hy - h_rad, hx + h_rad, hy + h_rad], fill=COLOR_HIGHLIGHT
            )

    return img


def render_icon(size: int) -> Image.Image:
    """统一渲染接口: 优先 cairosvg, 失败则 PIL"""
    try:
        return render_with_cairosvg(size)
    except (ImportError, RuntimeError):
        return render_with_pil(size)


def generate_ico(output_path: Path) -> bool:
    """生成 Windows .ico (含多尺寸)"""
    try:
        images = [render_icon(s) for s in ICO_SIZES]
        # PIL 要求所有图像为 RGBA
        images = [img.convert("RGBA") for img in images]
        images[0].save(
            output_path,
            format="ICO",
            sizes=[(s, s) for s in ICO_SIZES],
            append_images=images[1:],
            bitmap_format="png",
        )
        return True
    except Exception as e:
        print(f"  ⚠️  ICO 生成失败: {type(e).__name__}")
        return False


def generate_icns(output_path: Path) -> bool:
    """生成 macOS .icns (需 icnsutil)"""
    try:
        import icnsutil
    except ImportError:
        print("  ⚠️  跳过 .icns: 未安装 icnsutil (pip install icnsutil)")
        return False

    try:
        icns = icnsutil.ICNSFile()
        for s in ICNS_SIZES:
            img = render_icon(s)
            icns.add_icon(img, s)
        icns.write(output_path)
        return True
    except Exception as e:
        print(f"  ⚠️  ICNS 生成失败: {type(e).__name__}")
        return False


def generate_android_icons() -> bool:
    """生成 Android 图标集 (ic_launcher, ic_launcher_round, ic_launcher_foreground)"""
    if not ANDROID_OUTPUT_DIR.exists():
        print(f"  ⚠️  Android 目录不存在: {ANDROID_OUTPUT_DIR}")
        print("     请先运行: pnpm tauri android init")
        return False

    try:
        success_count = 0
        total_count = 0

        for density, size in ANDROID_SIZES.items():
            mipmap_dir = ANDROID_OUTPUT_DIR / f"mipmap-{density}"

            # 创建目录（如果不存在）
            mipmap_dir.mkdir(parents=True, exist_ok=True)

            # 渲染图标
            img = render_icon(size)

            # 生成 ic_launcher.png
            launcher_path = mipmap_dir / "ic_launcher.png"
            img.save(launcher_path, "PNG")
            success_count += 1
            total_count += 1
            print(f"  ✓ {density}/ic_launcher.png ({size}x{size})")

            # 生成 ic_launcher_round.png (圆形版本，使用同一图标)
            round_path = mipmap_dir / "ic_launcher_round.png"
            img.save(round_path, "PNG")
            success_count += 1
            total_count += 1
            print(f"  ✓ {density}/ic_launcher_round.png ({size}x{size})")

            # 生成 ic_launcher_foreground.png (前景层，去掉透明背景)
            # 创建一个带白色背景的前景层
            foreground = Image.new("RGBA", (size, size), (255, 255, 255, 0))
            foreground.paste(img, (0, 0), img)  # 使用 alpha 通道作为掩码

            foreground_path = mipmap_dir / "ic_launcher_foreground.png"
            foreground.save(foreground_path, "PNG")
            success_count += 1
            total_count += 1
            print(f"  ✓ {density}/ic_launcher_foreground.png ({size}x{size})")

        print(f"\n  ✅ Android 图标生成完成: {success_count}/{total_count}")
        return True

    except Exception as e:
        print(f"  ❌ Android 图标生成失败: {type(e).__name__}: {e}")
        return False


def main():
    print(f"🚀 为 '{APP_NAME}' 生成 Tauri 图标集 (严格匹配目录结构)\n")
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # 1. 生成所有 PNG (按指定命名)
    print("🖼️  生成 PNG 图标:")
    png_success = 0
    for size, filename in ICON_SPECS:
        try:
            img = render_icon(size)
            img.save(OUTPUT_DIR / filename, "PNG")
            print(f"  ✓ {filename:25} ({size:3}x{size})")
            png_success += 1
        except Exception as e:
            print(f"  ❌ {filename}: {type(e).__name__}")

    # 2. 生成 icon.ico
    print("\n🪟 生成 Windows 图标:")
    ico_path = OUTPUT_DIR / "icon.ico"
    if generate_ico(ico_path):
        print(f"  ✓ icon.ico (含 {len(ICO_SIZES)} 个尺寸)")
    else:
        print(f"  ✗ icon.ico 生成失败")

    # 3. 生成 icon.icns
    print("\n🍎 生成 macOS 图标:")
    icns_path = OUTPUT_DIR / "icon.icns"
    if generate_icns(icns_path):
        print(f"  ✓ icon.icns (含 {len(ICNS_SIZES)} 个尺寸)")
    else:
        print(f"  ✗ icon.icns 生成跳过/失败")

    # 4. 生成 Android 图标
    print("\n📱 生成 Android 图标:")
    android_success = generate_android_icons()

    # 5. 验证与总结
    total_expected = len(ICON_SPECS) + 2  # + ICO + ICNS
    generated = (
        len(list(OUTPUT_DIR.glob("*.png")))
        + (1 if ico_path.exists() else 0)
        + (1 if icns_path.exists() else 0)
    )

    print("\n" + "=" * 55)
    print(f"✅ 成功生成 {generated}/{total_expected} 个桌面图标文件!")
    print(f"📁 桌面图标输出目录: {OUTPUT_DIR.resolve()}")

    if android_success:
        print(f"\n✅ Android 图标已生成!")
        print(f"📁 Android 图标输出目录: {ANDROID_OUTPUT_DIR.resolve()}")

    print("\n🔍 验证目录结构:")
    print("   tree src-tauri/icons")
    print("   tree src-tauri/gen/android/app/src/main/res/mipmap-*")
    print("\n💡 关键说明:")
    print("  • 128x128@2x.png = 256x256 (行业标准命名)")
    print("  • StoreLogo.png = 50x50 (符合 Windows Store 要求)")
    print("  • 小尺寸 (<48px) 已智能简化细节，确保清晰度")
    print("  • 透明背景，系统自动添加圆角/阴影")
    print("\n📱 Android 图标说明:")
    print("  • ic_launcher.png: 标准应用图标")
    print("  • ic_launcher_round.png: 圆形图标 (某些启动器使用)")
    print("  • ic_launcher_foreground.png: 自适应图标前景层")
    print("  • 支持的密度: mdpi, hdpi, xhdpi, xxhdpi, xxxhdpi")
    print("\n⚙️  Tauri 配置建议 (tauri.conf.json):")
    print(
        '  "bundle": { "icon": ["icons/icon.png", "icons/icon.ico", "icons/icon.icns"] }'
    )
    print("\n⚠️  注意: Android 图标每次运行 'pnpm tauri android init' 后需要重新生成")
    print("=" * 55)


if __name__ == "__main__":
    # 检查 Tauri 项目结构
    if not Path("src-tauri").exists():
        print("⚠️  提示: 未检测到 src-tauri/ 目录")
        print("   请在 Tauri 项目根目录运行此脚本")
        print(f"   当前目录: {Path.cwd()}")

    main()
