from struct import pack
import zlib
import math
import os


def create_png_rgba(width, height, pixels):
    """Create a PNG from RGBA pixel data (list of (r,g,b,a) per pixel, row by row)."""

    def chunk(chunk_type, data):
        c = chunk_type + data
        return pack(">I", len(data)) + c + pack(">I", zlib.crc32(c) & 0xFFFFFFFF)

    header = b"\x89PNG\r\n\x1a\n"
    # color type 6 = RGBA
    ihdr = chunk(b"IHDR", pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))

    raw = b""
    for y in range(height):
        raw += b"\x00"  # filter byte
        for x in range(width):
            r, g, b, a = pixels[y * width + x]
            raw += bytes((r, g, b, a))

    idat = chunk(b"IDAT", zlib.compress(raw))
    iend = chunk(b"IEND", b"")
    return header + ihdr + idat + iend


def create_ico(png_data_256):
    """Create a minimal ICO file embedding a 256x256 PNG."""
    header = pack("<HHH", 0, 1, 1)
    entry = pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(png_data_256), 22)
    return header + entry + png_data_256


def render_icon(size):
    """
    Render a TunnelHub icon at the given size.
    Icon design: rounded green square background with a white tunnel/link symbol.
    """
    pixels = []
    cx, cy = size / 2, size / 2
    radius = size * 0.42  # Background rounded-rect corner radius
    bg = (24, 160, 88)  # Green brand color #18A058

    # Helper: distance to rounded rect border (for anti-aliasing)
    def in_rounded_rect(px, py, x0, y0, x1, y1, r):
        """Return coverage 0..1 for a rounded rect."""
        # Clamp to corner regions
        dx = max(x0 + r - px, 0, px - (x1 - r))
        dy = max(y0 + r - py, 0, py - (y1 - r))
        if dx > 0 and dy > 0:
            dist = math.sqrt(dx * dx + dy * dy) - r
        else:
            dist = -1  # fully inside
        if dist < -1:
            return 1.0
        elif dist > 1:
            return 0.0
        else:
            return max(0.0, min(1.0, 0.5 - dist * 0.5))

    pad = size * 0.04
    corner_r = size * 0.18

    for y in range(size):
        for x in range(size):
            # Background rounded rectangle
            bg_cov = in_rounded_rect(x + 0.5, y + 0.5, pad, pad, size - pad, size - pad, corner_r)

            if bg_cov < 0.01:
                pixels.append((0, 0, 0, 0))
                continue

            # Draw the tunnel symbol: a horizontal link/arrow shape
            # Two circles connected by parallel lines (like a tunnel entrance)
            fx = (x + 0.5 - cx) / (size * 0.35)  # normalized coords
            fy = (y + 0.5 - cy) / (size * 0.35)

            white = False

            # Left circle (tunnel entrance)
            ldx, ldy = fx + 0.55, fy
            ld = math.sqrt(ldx * ldx + ldy * ldy)
            if 0.32 < ld < 0.52:
                white = True

            # Right circle (tunnel exit)
            rdx, rdy = fx - 0.55, fy
            rd = math.sqrt(rdx * rdx + rdy * rdy)
            if 0.32 < rd < 0.52:
                white = True

            # Connecting horizontal bars (top and bottom)
            if -0.55 < fx < 0.55 and 0.32 < abs(fy) < 0.52:
                white = True

            # Arrow head pointing right
            ax = fx - 0.65
            ay = abs(fy)
            if 0 < ax < 0.35 and ay < (0.35 - ax) * 0.8 and ay < 0.25:
                white = True

            if white:
                r_c = int(bg[0] * (1 - bg_cov) + 255 * bg_cov)
                g_c = int(bg[1] * (1 - bg_cov) + 255 * bg_cov)
                b_c = int(bg[2] * (1 - bg_cov) + 255 * bg_cov)
                pixels.append((r_c, g_c, b_c, int(bg_cov * 255)))
            else:
                pixels.append((bg[0], bg[1], bg[2], int(bg_cov * 255)))

    return pixels


icons_dir = os.path.join("src-tauri", "icons")
os.makedirs(icons_dir, exist_ok=True)

# 32x32
pixels32 = render_icon(32)
png32 = create_png_rgba(32, 32, pixels32)
with open(os.path.join(icons_dir, "32x32.png"), "wb") as f:
    f.write(png32)

# 128x128
pixels128 = render_icon(128)
png128 = create_png_rgba(128, 128, pixels128)
with open(os.path.join(icons_dir, "128x128.png"), "wb") as f:
    f.write(png128)

# 256x256 (for @2x)
pixels256 = render_icon(256)
png256 = create_png_rgba(256, 256, pixels256)
with open(os.path.join(icons_dir, "128x128@2x.png"), "wb") as f:
    f.write(png256)

# ICO file (embeds 256x256 PNG)
ico = create_ico(png256)
with open(os.path.join(icons_dir, "icon.ico"), "wb") as f:
    f.write(ico)

# ICNS placeholder
with open(os.path.join(icons_dir, "icon.icns"), "wb") as f:
    f.write(png256)

print("Icons created successfully")
