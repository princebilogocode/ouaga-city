"""Draw the foliage texture (a cluster of leaves on transparent background) used on the tree
leaf cards: assets/tex/feuillage.png (RGBA, 512 px). Hundreds of small leaves in Sahel greens,
lit from above, with twigs."""
import math, os, random
from PIL import Image, ImageDraw, ImageFilter

APP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
S = 1024
random.seed(11)
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
c = S / 2
# twigs from the centre
for k in range(14):
    a = random.uniform(0, math.tau)
    l = random.uniform(0.25, 0.46) * S
    x1, y1 = c + math.cos(a) * l, c + math.sin(a) * l
    d.line([(c, c), (x1, y1)], fill=(70, 52, 38, 255), width=5)
leaves = []
for k in range(1500):
    # denser in the middle, ragged outline
    r = (random.random() ** 0.75) * S * 0.47
    a = random.uniform(0, math.tau)
    x, y = c + math.cos(a) * r, c + math.sin(a) * r * 0.92
    leaves.append((y, x, a))
leaves.sort()  # lower leaves drawn last look closer (lit from above)
for y, x, a in leaves:
    L = random.uniform(26, 44)
    W = L * random.uniform(0.32, 0.45)
    ang = random.uniform(0, math.tau)
    top = 1.0 - (y / S)  # lighter at the top
    shade = random.uniform(0.55, 1.0) * (0.75 + 0.35 * top)
    base = random.choice([(78, 118, 46), (92, 132, 52), (64, 104, 40), (108, 140, 60), (84, 112, 38)])
    col = tuple(min(255, int(ch * shade)) for ch in base) + (255,)
    pts = []
    for t in range(14):
        u = t / 13 * math.pi
        px = math.cos(u) * L / 2
        py = math.sin(u) * W / 2
        pts.append((px, py))
    pts += [(-px, -py) for px, py in pts[1:-1]]
    ca, sa = math.cos(ang), math.sin(ang)
    poly = [(x + px * ca - py * sa, y + px * sa + py * ca) for px, py in pts]
    d.polygon(poly, fill=col)
    # midrib
    d.line([(x - L / 2 * ca, y - L / 2 * sa), (x + L / 2 * ca, y + L / 2 * sa)], fill=tuple(min(255, int(ch * 1.18)) for ch in col[:3]) + (255,), width=1)
img = img.filter(ImageFilter.SMOOTH)
# alpha: hard cut so the game can cut it out cleanly
r, g, b, a = img.split()
a = a.point(lambda v: 255 if v > 110 else 0)
img = Image.merge("RGBA", (r, g, b, a)).resize((512, 512), Image.LANCZOS)
# bleed colours under transparent pixels (no dark fringes when the texture is filtered)
px = img.load()
avg = (80, 118, 46)
for yy in range(512):
    for xx in range(512):
        if px[xx, yy][3] < 128:
            px[xx, yy] = avg + (0,)
img.save(os.path.join(APP, "assets", "tex", "feuillage.png"))
print("ok")
