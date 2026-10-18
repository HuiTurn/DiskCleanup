#!/usr/bin/env python3
"""生成 DiskCleanup 应用图标 PNG（纯 stdlib，无第三方依赖）。"""
import struct, zlib, os, math

def png_chunk(tag, data):
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

def make_icon(size):
    px = bytearray()
    cx = cy = size / 2
    r = size * 0.46
    r2 = size * 0.30
    for y in range(size):
        px.append(0)  # filter byte
        for x in range(size):
            dx, dy = x - cx, y - cy
            d = math.hypot(dx, dy)
            if d <= r:
                # 渐变：左上亮蓝 -> 右下深蓝
                t = (dx + dy + size) / (2 * size)
                cr = int(47 + (30 - 47) * t)
                cg = int(123 + (99 - 123) * t)
                cb = int(246 + (214 - 246) * t)
                # 白色圆环（代表磁盘/清理环）
                if r2 * 0.72 <= d <= r2:
                    cr = cg = cb = 255
                elif d <= r2 * 0.72:
                    cr, cg, cb = 255, 255, 255
                px += bytes((cr, cg, cb, 255))
            else:
                px += bytes((0, 0, 0, 0))
    return px

def write_png(path, size):
    raw = make_icon(size)
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    data = png_chunk(b"IHDR", ihdr) + png_chunk(b"IDAT", zlib.compress(raw, 9)) + png_chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + data)

out = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons")
os.makedirs(out, exist_ok=True)
for name, size in [("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256), ("icon.png", 1024)]:
    write_png(os.path.join(out, name), size)
    print("wrote", name)
