# Blender script: the trees of Ouaga (neem, mango tree, flamboyant, baobab, palm), modelled with
# real branching trunks and leaf cards (the foliage texture is cut out in the game).
# Usage: blender -b -P build_trees.py -- <out_dir>
# For each species: <name>.ocs (near, detailed) and <name>_lod.ocs (far away, a few crossed cards).
# Size: 1 unit = 1 m, the trunk stands at the origin; the game scales each tree to its map size.
import bpy, sys, os, math, random
from mathutils import Vector, Matrix

sys.path.append(os.path.dirname(__file__))
from ocs_export import export, mat

OUT = sys.argv[sys.argv.index("--") + 1]
BARK = "L12_150_130_112"
BARK_DARK = "L12_120_104_92"
BARK_BAOBAB = "L12_178_160_146"


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def limb(parts, a, b, r0, r1, material, verts=7):
    a, b = Vector(a), Vector(b)
    d = b - a
    bpy.ops.mesh.primitive_cone_add(vertices=verts, radius1=r0, radius2=r1, depth=d.length, location=(a + b) / 2)
    o = bpy.context.active_object
    o.rotation_mode = "QUATERNION"
    o.rotation_quaternion = Vector((0, 0, 1)).rotation_difference(d.normalized())
    o.data.materials.append(mat(material))
    for p in o.data.polygons:
        p.use_smooth = True
    parts.append(o)
    return o


def card(parts, c, size, material, rng, up_bias=0.5, stretch=1.0):
    bpy.ops.mesh.primitive_plane_add(size=1.0, location=c)
    o = bpy.context.active_object
    o.scale = (size * stretch, size, 1)
    # random orientation, mostly facing up/outwards so the crown reads full from every side
    o.rotation_euler = (rng.uniform(-1.3, 1.3) * (1 - up_bias), rng.uniform(-1.3, 1.3) * (1 - up_bias), rng.uniform(0, math.tau))
    o.data.materials.append(mat(material))
    parts.append(o)
    return o


def leaf_mat(rng, base=(255, 255, 255)):
    k = rng.uniform(0.82, 1.05)
    return "L9_%d_%d_%d_UV" % tuple(min(255, int(v * k)) for v in base)


def branch_tree(parts, rng, trunk_h, trunk_r, n_main, spread, rise, length, sub, crown_cards, card_size, flat=0.0, leaf_tint=(255, 255, 255)):
    """A trunk that forks into main branches, each with sub-branches ending in leaf clusters."""
    top = Vector((rng.uniform(-0.15, 0.15), rng.uniform(-0.15, 0.15), trunk_h))
    limb(parts, (0, 0, 0), top, trunk_r, trunk_r * 0.72, BARK, 9)
    # root flare
    limb(parts, (0, 0, -0.1), (0, 0, 0.5), trunk_r * 1.35, trunk_r, BARK, 9)
    tips = []
    for i in range(n_main):
        a = i / n_main * math.tau + rng.uniform(-0.3, 0.3)
        d = Vector((math.cos(a) * spread, math.sin(a) * spread, rise + rng.uniform(-0.15, 0.15))).normalized()
        end = top + d * length * rng.uniform(0.8, 1.1)
        limb(parts, top, end, trunk_r * 0.6, trunk_r * 0.3, BARK)
        for j in range(sub):
            t = rng.uniform(0.45, 1.0)
            s0 = top + (end - top) * t
            a2 = a + rng.uniform(-1.1, 1.1)
            d2 = Vector((math.cos(a2), math.sin(a2), rise * 0.8 + rng.uniform(-0.1, 0.4) - flat)).normalized()
            s1 = s0 + d2 * length * rng.uniform(0.4, 0.65)
            limb(parts, s0, s1, trunk_r * 0.22, trunk_r * 0.08, BARK_DARK, 5)
            tips.append(s1)
        tips.append(end)
    # leaf clusters around every tip, plus filling the crown volume
    lo = min(t.z for t in tips)
    hi = max(t.z for t in tips)
    centre = sum(tips, Vector()) / len(tips)
    for k in range(crown_cards):
        t = tips[k % len(tips)]
        off = Vector((rng.gauss(0, 0.7), rng.gauss(0, 0.7), rng.gauss(0.2, 0.45)))
        p = t + off
        if k % 4 == 0:
            p = centre + Vector((rng.gauss(0, 1.2), rng.gauss(0, 1.2), rng.gauss(0.3, 0.6)))
        card(parts, p, card_size * rng.uniform(0.75, 1.2), leaf_mat(rng, leaf_tint), rng, up_bias=0.35 + flat * 0.4)
    return centre, (hi - lo)


def lod(name, crown_c, crown_r, crown_h, trunk_h, trunk_r, tint=(255, 255, 255)):
    """Far away: the trunk and four crossed foliage cards."""
    reset()
    parts = []
    limb(parts, (0, 0, 0), (0, 0, trunk_h + crown_h * 0.3), trunk_r, trunk_r * 0.6, BARK, 5)
    for k in range(3):
        bpy.ops.mesh.primitive_plane_add(size=1.0, location=crown_c)
        o = bpy.context.active_object
        o.scale = (crown_r * 2.2, crown_h * 1.3, 1)
        o.rotation_euler = (math.pi / 2, 0, k * math.pi / 3)
        o.data.materials.append(mat("L9_%d_%d_%d_UV" % tint))
        parts.append(o)
    bpy.ops.mesh.primitive_plane_add(size=1.0, location=(crown_c[0], crown_c[1], crown_c[2] + crown_h * 0.15))
    o = bpy.context.active_object
    o.scale = (crown_r * 2.1, crown_r * 2.1, 1)
    o.data.materials.append(mat("L9_%d_%d_%d_UV" % tint))
    parts.append(o)
    export(parts, os.path.join(OUT, name + "_lod.ocs"), apply_mods=False)


def neem():
    reset(); rng = random.Random(3); parts = []
    c, h = branch_tree(parts, rng, 2.6, 0.24, 5, 0.75, 0.75, 2.6, 3, 70, 1.7)
    export(parts, os.path.join(OUT, "neem.ocs"))
    lod("neem", (0, 0, 5.4), 3.2, 3.4, 2.6, 0.24)


def manguier():
    reset(); rng = random.Random(5); parts = []
    branch_tree(parts, rng, 2.0, 0.32, 6, 0.9, 0.55, 3.3, 3, 95, 1.9, leaf_tint=(215, 230, 200))
    export(parts, os.path.join(OUT, "manguier.ocs"))
    lod("manguier", (0, 0, 4.6), 4.0, 3.6, 2.0, 0.32, (215, 230, 200))


def flamboyant():
    reset(); rng = random.Random(9); parts = []
    branch_tree(parts, rng, 3.0, 0.26, 5, 1.0, 0.28, 3.8, 3, 75, 1.9, flat=0.25, leaf_tint=(240, 255, 225))
    # a few clusters of red flowers, as in the hot season
    for k in range(14):
        a = rng.uniform(0, math.tau); r = rng.uniform(1.5, 4.0)
        card(parts, (math.cos(a) * r, math.sin(a) * r, 4.6 + rng.uniform(-0.3, 0.5)), 0.9, "L9_255_120_90_UV", rng, up_bias=0.8)
    export(parts, os.path.join(OUT, "flamboyant.ocs"))
    lod("flamboyant", (0, 0, 4.4), 4.5, 2.2, 3.0, 0.26, (240, 255, 225))


def baobab():
    reset(); rng = random.Random(13); parts = []
    # a huge bottle trunk, then stubby branches with sparse leaves
    limb(parts, (0, 0, -0.1), (0, 0, 2.2), 1.25, 1.1, BARK_BAOBAB, 14)
    limb(parts, (0, 0, 2.2), (0, 0, 5.2), 1.1, 0.7, BARK_BAOBAB, 14)
    top = Vector((0, 0, 5.2))
    tips = []
    for i in range(7):
        a = i / 7 * math.tau + rng.uniform(-0.2, 0.2)
        d = Vector((math.cos(a), math.sin(a), rng.uniform(0.3, 0.8))).normalized()
        end = top + d * rng.uniform(2.2, 3.4)
        limb(parts, top - Vector((0, 0, 0.4)), end, 0.35, 0.14, BARK_BAOBAB, 8)
        for j in range(2):
            a2 = a + rng.uniform(-0.9, 0.9)
            e2 = end + Vector((math.cos(a2), math.sin(a2), rng.uniform(0.2, 0.7))).normalized() * rng.uniform(1.0, 1.6)
            limb(parts, end, e2, 0.13, 0.05, BARK_BAOBAB, 5)
            tips.append(e2)
    for k in range(34):
        t = tips[k % len(tips)]
        card(parts, t + Vector((rng.gauss(0, 0.4), rng.gauss(0, 0.4), rng.gauss(0.1, 0.3))), 1.3, leaf_mat(rng, (225, 240, 205)), rng, up_bias=0.4)
    export(parts, os.path.join(OUT, "baobab.ocs"))
    lod("baobab", (0, 0, 6.8), 3.6, 2.4, 5.2, 1.1, (225, 240, 205))


def palmier():
    reset(); rng = random.Random(17); parts = []
    # slightly curved ringed trunk
    pts = [Vector((0.08 * math.sin(i * 0.6), 0.05 * i * 0.1, i * 0.9)) for i in range(11)]
    for i in range(10):
        limb(parts, pts[i], pts[i + 1], 0.22 - i * 0.008, 0.21 - i * 0.008, BARK, 8)
    top = pts[-1]
    # fronds: long arched cards
    for k in range(16):
        a = k / 16 * math.tau + rng.uniform(-0.1, 0.1)
        droop = rng.uniform(-0.6, 0.35)
        for s in range(3):
            r = 0.9 + s * 1.05
            p = top + Vector((math.cos(a) * r, math.sin(a) * r, droop * s * 0.8 + 0.25 - 0.12 * s * s))
            bpy.ops.mesh.primitive_plane_add(size=1.0, location=p)
            o = bpy.context.active_object
            o.scale = (1.25, 0.55, 1)
            o.rotation_euler = (0, -(droop * 0.8 - 0.3 * s) , a)
            o.data.materials.append(mat(leaf_mat(rng, (230, 245, 200))))
            parts.append(o)
    export(parts, os.path.join(OUT, "palmier.ocs"))
    lod("palmier", (0, 0, 9.4), 3.0, 1.6, 8.6, 0.2, (230, 245, 200))


os.makedirs(OUT, exist_ok=True)
neem()
manguier()
flamboyant()
baobab()
palmier()
