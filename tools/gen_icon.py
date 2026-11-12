"""Génère l'icône B (« +1 ») : icon.ico (16 à 256 px) et icon_64.rgba (pixels bruts)."""
import sys
from PIL import Image, ImageDraw

OUT = sys.argv[1]
BG = (0x1E, 0x1E, 0x1E, 255)
ORANGE = (0xFF, 0xA5, 0x00, 255)
WHITE = (255, 255, 255, 255)


def line(d, s, p1, p2, w, color):
    """Trait à bouts ronds, coordonnées sur une grille de 64."""
    k = s / 64
    a = (p1[0] * k, p1[1] * k)
    b = (p2[0] * k, p2[1] * k)
    d.line([a, b], fill=color, width=round(w * k))
    r = w * k / 2
    for x, y in (a, b):
        d.ellipse([x - r, y - r, x + r, y + r], fill=color)


def draw(size, detail):
    """Dessine à 8x la taille voulue puis réduit (anti-crénelage)."""
    s = size * 8
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    k = s / 64
    d.rounded_rectangle([2 * k, 2 * k, 62 * k, 62 * k], radius=12 * k, fill=BG)
    if detail == "full":
        w = 6
        line(d, s, (12, 32), (28, 32), w, ORANGE)
        line(d, s, (20, 24), (20, 40), w, ORANGE)
        line(d, s, (37, 22), (44, 17), w, WHITE)
        line(d, s, (44, 17), (44, 47), w, WHITE)
        line(d, s, (37, 47), (51, 47), w, WHITE)
    elif detail == "medium":
        w = 7
        line(d, s, (11, 32), (29, 32), w, ORANGE)
        line(d, s, (20, 23), (20, 41), w, ORANGE)
        line(d, s, (36, 23), (44, 17), w, WHITE)
        line(d, s, (44, 17), (44, 47), w, WHITE)
    else:  # small : un « + » et une barre, très épais
        w = 9
        line(d, s, (11, 32), (29, 32), w, ORANGE)
        line(d, s, (20, 23), (20, 41), w, ORANGE)
        line(d, s, (44, 15), (44, 49), w, WHITE)
    return img.resize((size, size), Image.LANCZOS)


sizes = {16: "small", 20: "small", 24: "small", 32: "medium", 40: "medium",
         48: "full", 64: "full", 128: "full", 256: "full"}
images = {n: draw(n, det) for n, det in sizes.items()}
images[256].save(f"{OUT}/icon.ico", format="ICO",
                 sizes=[(n, n) for n in sizes],
                 append_images=[images[n] for n in sizes if n != 256])
# Le format ICO de Pillow redimensionne la plus grande image : on réécrit chaque taille
# à la main pour garder les versions simplifiées des petites tailles.
import io, struct
entries, blobs = [], []
for n in sorted(sizes):
    buf = io.BytesIO()
    images[n].save(buf, format="PNG")
    blobs.append(buf.getvalue())
    entries.append(n)
header = struct.pack("<HHH", 0, 1, len(entries))
offset = 6 + 16 * len(entries)
dirs = b""
for n, blob in zip(entries, blobs):
    dim = 0 if n == 256 else n
    dirs += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(blob), offset)
    offset += len(blob)
with open(f"{OUT}/icon.ico", "wb") as f:
    f.write(header + dirs + b"".join(blobs))
with open(f"{OUT}/icon_64.rgba", "wb") as f:
    f.write(images[64].tobytes())
# Aperçu pour vérification (hors dépôt).
preview = Image.new("RGBA", (420, 140), (60, 60, 60, 255))
x = 10
for n in (256, 64, 48, 32, 24, 16):
    im = images[n] if n != 256 else images[256].resize((120, 120), Image.LANCZOS)
    preview.paste(im, (x, 10), im)
    x += im.size[0] + 10
preview.save(sys.argv[2])
print("ok")
