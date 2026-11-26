# Blender script: the building kit used to dress every building of the city in 3D.
# Each piece is exported in its own frame: x along the wall, y up, z out of the wall (game space),
# origin at the bottom centre of the piece, on the wall plane.
# In Blender: x along the wall, z up, the outside of the wall is -y.
# PAINT (magenta) faces take the building's accent colour in the game.
# Usage: blender -b -P build_kit.py -- <out_dir>
import bpy, sys, os, math

sys.path.append(os.path.dirname(__file__))
from ocs_export import export, mat

OUT = sys.argv[sys.argv.index("--") + 1]

FRAME = "L8_232_230_224"
GLASS = "L6_46_60_72"
METAL = "L8_45_48_50"
GRILLE = "L8_38_60_48"
CONCRETE = "L5_210_205_196"
SIGN_WHITE = "L8_240_238_230"


def clear():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def box(parts, c, s, material, bevel=0.0, rot=None):
    bpy.ops.mesh.primitive_cube_add(size=1, location=c, rotation=rot or (0, 0, 0))
    o = bpy.context.active_object
    o.scale = s
    bpy.ops.object.transform_apply(scale=True)
    o.data.materials.append(mat(material))
    if bevel > 0 and False:  # light pieces: thousands of them are placed around the camera
        b = o.modifiers.new("b", "BEVEL")
        b.width = bevel
        b.segments = 2
    parts.append(o)
    return o


def cyl(parts, c, r, h, material, verts=16, rot=None, r2=None):
    bpy.ops.mesh.primitive_cone_add(radius1=r, radius2=r if r2 is None else r2, depth=h, location=c, vertices=min(verts, 10), rotation=rot or (0, 0, 0))
    o = bpy.context.active_object
    o.data.materials.append(mat(material))
    for p in o.data.polygons:
        p.use_smooth = True
    parts.append(o)
    return o


def window(name, w=1.25, h=1.35, shutters=False):
    clear()
    parts = []
    d = 0.09
    box(parts, (0, -0.012, h / 2), (w - 0.06, 0.012, h - 0.06), GLASS)
    for sx in (-1, 1):
        box(parts, (sx * (w / 2 - 0.04), -d / 2, h / 2), (0.08, d, h), FRAME, 0.01)
    box(parts, (0, -d / 2, h - 0.04), (w, d, 0.08), FRAME, 0.01)
    box(parts, (0, -d / 2, 0.04), (w, d, 0.08), FRAME, 0.01)
    box(parts, (0, -d / 2, h / 2), (0.05, d * 0.8, h - 0.1), FRAME)
    # sill and lintel
    box(parts, (0, -0.1, -0.04), (w + 0.25, 0.2, 0.08), CONCRETE, 0.015)
    box(parts, (0, -0.05, h + 0.06), (w + 0.3, 0.1, 0.12), CONCRETE, 0.01)
    if shutters:
        # louvred metal shutters, half open
        for sx in (-1, 1):
            cx = sx * (w / 2 + 0.3)
            box(parts, (cx, -0.04, h / 2), (0.58, 0.035, h), "PAINT", 0.008)
            for k in range(9):
                z = 0.12 + k * (h - 0.24) / 8
                box(parts, (cx, -0.065, z), (0.52, 0.02, 0.035), "PAINT", 0.0, (math.radians(-25), 0, 0))
    else:
        # anti-theft grille, as on most houses in Ouaga
        for k in range(6):
            x = -w / 2 + 0.12 + k * (w - 0.24) / 5
            box(parts, (x, -0.11, h / 2), (0.022, 0.022, h - 0.12), GRILLE)
        for z in (h * 0.33, h * 0.66):
            box(parts, (0, -0.11, z), (w - 0.1, 0.02, 0.025), GRILLE)
    export(parts, os.path.join(OUT, name + ".ocs"), uv_scale=1.0)


def door():
    clear()
    parts = []
    w, h = 1.0, 2.15
    box(parts, (0, -0.03, h / 2), (w, 0.05, h), "PAINT", 0.01)
    # pressed panels
    for z in (0.55, 1.55):
        box(parts, (0, -0.06, z), (w - 0.25, 0.015, 0.7), "PAINT", 0.01)
    box(parts, (0.36, -0.08, 1.05), (0.12, 0.03, 0.04), "L8_200_200_205")
    for sx in (-1, 1):
        box(parts, (sx * (w / 2 + 0.05), -0.05, h / 2), (0.1, 0.1, h + 0.1), CONCRETE, 0.01)
    box(parts, (0, -0.05, h + 0.07), (w + 0.2, 0.1, 0.14), CONCRETE, 0.01)
    box(parts, (0, -0.25, 0.06), (w + 0.4, 0.5, 0.12), CONCRETE, 0.015)
    # small tin awning over the door
    box(parts, (0, -0.45, h + 0.32), (w + 0.6, 0.9, 0.03), "L4_210_212_216", 0.0, (math.radians(-12), 0, 0))
    for sx in (-1, 1):
        box(parts, (sx * (w / 2 + 0.25), -0.45, h + 0.2), (0.03, 0.8, 0.03), METAL, 0.0, (math.radians(-12), 0, 0))
    export(parts, os.path.join(OUT, "porte.ocs"), uv_scale=1.0)


def gate():
    clear()
    parts = []
    w, h = 3.0, 2.2
    for sx in (-1, 1):
        box(parts, (sx * w / 4, -0.04, h / 2), (w / 2 - 0.03, 0.04, h), "PAINT", 0.008)
        for k in range(4):
            box(parts, (sx * w / 4, -0.07, 0.3 + k * 0.5), (w / 2 - 0.2, 0.02, 0.06), "PAINT")
        # pointed bars on top
        for k in range(6):
            x = sx * (0.12 + k * 0.23)
            cyl(parts, (x, -0.04, h + 0.15), 0.015, 0.3, METAL, 6)
            cyl(parts, (x, -0.04, h + 0.33), 0.03, 0.06, METAL, 6, None, 0.0)
    for sx in (-1, 1):
        box(parts, (sx * (w / 2 + 0.2), -0.15, (h + 0.3) / 2), (0.4, 0.4, h + 0.3), CONCRETE, 0.02)
        cyl(parts, (sx * (w / 2 + 0.2), -0.15, h + 0.38), 0.18, 0.16, "PAINT", 12, None, 0.08)
    export(parts, os.path.join(OUT, "portail.ocs"), uv_scale=1.0)


def shopfront():
    clear()
    parts = []
    w, h = 3.6, 2.6
    # opening with a half rolled-up shutter and a dark interior
    box(parts, (0, -0.01, h / 2), (w - 0.3, 0.02, h), "L8_30_26_24")
    box(parts, (0, -0.05, h - 0.35), (w - 0.2, 0.06, 0.7), "L4_205_205_210")
    cyl(parts, (0, -0.12, h + 0.05), 0.14, w - 0.2, "L4_190_190_195", 14, (0, math.pi / 2, 0))
    for sx in (-1, 1):
        box(parts, (sx * (w / 2 - 0.08), -0.08, h / 2), (0.16, 0.16, h), CONCRETE, 0.01)
    # counter with goods (bottles, crates, sacks)
    box(parts, (0, -0.45, 0.45), (w - 0.6, 0.7, 0.9), "L19_200_170_130", 0.02)
    for k in range(10):
        x = -1.3 + k * 0.29
        col = ["L6_40_110_50", "L6_150_40_30", "L6_220_170_40", "L8_235_235_230"][k % 4]
        cyl(parts, (x, -0.55, 1.03), 0.045, 0.26, col, 8, None, 0.03)
    box(parts, (1.1, -0.5, 1.05), (0.45, 0.35, 0.25), "L8_200_60_40", 0.02)
    box(parts, (-1.2, -0.3, 1.05), (0.5, 0.3, 0.3), "L18_220_210_180", 0.06)
    # tin awning on wooden posts
    box(parts, (0, -1.05, h + 0.25), (w + 0.4, 2.1, 0.03), "L4_255_255_255", 0.0, (math.radians(-10), 0, 0))
    box(parts, (0, -2.08, h + 0.07), (w + 0.4, 0.05, 0.22), "PAINT")
    for sx in (-1, 1):
        cyl(parts, (sx * (w / 2 + 0.1), -2.0, (h + 0.05) / 2), 0.05, h + 0.05, "L19_150_110_80", 8)
    # signboard above
    box(parts, (0, -0.06, h + 0.75), (w, 0.08, 0.75), SIGN_WHITE, 0.01)
    box(parts, (0, -0.105, h + 0.75), (w - 0.25, 0.01, 0.55), "PAINT")
    for k in range(3):
        box(parts, (-0.9 + k * 0.9, -0.115, h + 0.8), (0.6, 0.01, 0.12), SIGN_WHITE)
    box(parts, (0, -0.115, h + 0.6), (2.2, 0.01, 0.06), SIGN_WHITE)
    export(parts, os.path.join(OUT, "boutique.ocs"), uv_scale=1.0)


def balcony():
    clear()
    parts = []
    w, d = 3.0, 1.1
    box(parts, (0, -d / 2, -0.08), (w, d, 0.16), CONCRETE, 0.02)
    for k in range(9):
        x = -w / 2 + 0.08 + k * (w - 0.16) / 8
        cyl(parts, (x, -d + 0.06, 0.45), 0.02, 0.9, METAL, 6)
    for sx in (-1, 1):
        for k in range(3):
            y = -0.15 - k * 0.35
            cyl(parts, (sx * (w / 2 - 0.06), y, 0.45), 0.02, 0.9, METAL, 6)
    box(parts, (0, -d + 0.06, 0.92), (w, 0.06, 0.05), "PAINT", 0.01)
    for sx in (-1, 1):
        box(parts, (sx * (w / 2 - 0.06), -d / 2, 0.92), (0.06, d, 0.05), "PAINT", 0.01)
    # door-window to the balcony
    box(parts, (0, -0.012, 1.05), (1.1, 0.012, 2.0), GLASS)
    box(parts, (0, -0.04, 2.08), (1.25, 0.08, 0.08), FRAME)
    for sx in (-1, 1):
        box(parts, (sx * 0.58, -0.04, 1.05), (0.08, 0.08, 2.1), FRAME)
    # a potted plant and laundry, life on the balcony
    cyl(parts, (1.1, -0.6, 0.2), 0.14, 0.3, "L8_170_90_55", 12, None, 0.11)
    bpy.ops.mesh.primitive_uv_sphere_add(radius=0.22, location=(1.1, -0.6, 0.5), segments=10, ring_count=6)
    o = bpy.context.active_object
    o.data.materials.append(mat("L9_220_235_200"))
    parts.append(o)
    box(parts, (-0.9, -0.7, 0.75), (0.5, 0.01, 0.45), "L14_255_255_255")
    export(parts, os.path.join(OUT, "balcon.ocs"), uv_scale=1.0)


def clim():
    clear()
    parts = []
    box(parts, (0, -0.17, 0.27), (0.82, 0.32, 0.55), "L8_236_236_232", 0.03)
    cyl(parts, (0.15, -0.34, 0.27), 0.19, 0.02, "L8_120_122_126", 20, (math.pi / 2, 0, 0))
    for k in range(4):
        box(parts, (0.15, -0.35, 0.15 + k * 0.08), (0.38, 0.01, 0.015), "L8_90_92_96")
    box(parts, (0, -0.2, -0.05), (0.9, 0.4, 0.03), METAL)
    export(parts, os.path.join(OUT, "clim.ocs"), uv_scale=1.0)


def citerne():
    clear()
    parts = []
    # black polyethylene water tank on a metal stand (roofs of Ouaga)
    for sx in (-1, 1):
        for sy in (-1, 1):
            box(parts, (sx * 0.5, sy * 0.5, 0.4), (0.06, 0.06, 0.8), METAL)
    box(parts, (0, 0, 0.82), (1.2, 1.2, 0.05), METAL)
    cyl(parts, (0, 0, 1.45), 0.6, 1.2, "L8_32_32_34", 24)
    cyl(parts, (0, 0, 2.1), 0.6, 0.12, "L8_32_32_34", 24, None, 0.45)
    cyl(parts, (0, 0, 2.2), 0.2, 0.08, "L8_32_32_34", 16)
    export(parts, os.path.join(OUT, "citerne.ocs"), uv_scale=1.0)


def parabole():
    clear()
    parts = []
    cyl(parts, (0, 0, 0.5), 0.025, 1.0, METAL, 8)
    bpy.ops.mesh.primitive_uv_sphere_add(radius=0.45, location=(0, -0.2, 1.1), segments=16, ring_count=8)
    o = bpy.context.active_object
    o.scale = (1, 0.25, 1)
    o.rotation_euler = (math.radians(25), 0, 0)
    bpy.ops.object.transform_apply(scale=True, rotation=True)
    o.data.materials.append(mat("L8_225_225_222"))
    parts.append(o)
    cyl(parts, (0, -0.55, 1.2), 0.02, 0.5, METAL, 6, (math.radians(70), 0, 0))
    export(parts, os.path.join(OUT, "parabole.ocs"), uv_scale=1.0)


window("fenetre")
window("fenetre_volets", shutters=True)
door()
gate()
shopfront()
balcony()
clim()
citerne()
parabole()
