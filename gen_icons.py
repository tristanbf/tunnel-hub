from struct import pack
import zlib
import os


def create_png(width, height, color):
    def chunk(chunk_type, data):
        c = chunk_type + data
        return pack(">I", len(data)) + c + pack(">I", zlib.crc32(c) & 0xFFFFFFFF)

    header = b"\x89PNG\r\n\x1a\n"
    ihdr = chunk(b"IHDR", pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))

    raw = b""
    for y in range(height):
        raw += b"\x00"
        for x in range(width):
            raw += bytes(color)

    idat = chunk(b"IDAT", zlib.compress(raw))
    iend = chunk(b"IEND", b"")
    return header + ihdr + idat + iend


def create_ico(png_data):
    """Create a minimal ICO file from PNG data."""
    # ICO header: reserved(2) + type=1(2) + count=1(2)
    header = pack("<HHH", 0, 1, 1)
    # Entry: width=0(means 256), height=0, colors=0, reserved=0, planes=1, bpp=32, size, offset=22
    entry = pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(png_data), 22)
    return header + entry + png_data


icons_dir = os.path.join("src-tauri", "icons")
os.makedirs(icons_dir, exist_ok=True)

# 32x32
png32 = create_png(32, 32, (24, 160, 88))
with open(os.path.join(icons_dir, "32x32.png"), "wb") as f:
    f.write(png32)

# 128x128
png128 = create_png(128, 128, (24, 160, 88))
with open(os.path.join(icons_dir, "128x128.png"), "wb") as f:
    f.write(png128)

# 256x256 (for @2x)
png256 = create_png(256, 256, (24, 160, 88))
with open(os.path.join(icons_dir, "128x128@2x.png"), "wb") as f:
    f.write(png256)

# ICO file
ico = create_ico(png256)
with open(os.path.join(icons_dir, "icon.ico"), "wb") as f:
    f.write(ico)

# ICNS placeholder (just a copy)
with open(os.path.join(icons_dir, "icon.icns"), "wb") as f:
    f.write(png256)

print("Icons created successfully")
