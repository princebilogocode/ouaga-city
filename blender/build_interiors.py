# Blender script: model the interiors (maquis-café, Bila's apartment, gym, garage) with their furniture.
# Usage: blender -b -P build_interiors.py -- <out_dir>
# For each room: <name>.ocs (mesh) and <name>.json (size, door, colliders, interaction spots), game space.
import bpy, bmesh, sys, os, math, json, random

sys.path.append(os.path.dirname(__file__))
from ocs_export import export, mat

OUT = sys.argv[sys.argv.index("--") + 1]
random.seed(7)

PARQUET = "L16_255_255_255"
TILES = "L17_255_255_255"
WOOD = "L19_255_255_255"
WOOD_DARK = "L19_150_120_100"
LAMP = "EMIT_255_236_200"
WINDOW = "EMIT_205_228_255"
METAL = "L8_70_72_78"
CHROME = "L8_190_192_198"
BLACK = "L8_25_25_28"


class Room:
    def __init__(self, name, L, W, H, floor, wall, door_w=1.3):
        bpy.ops.wm.read_factory_settings(use_empty=True)
        self.name, self.L, self.W, self.H = name, L, W, H
        self.parts, self.colliders, self.spots = [], [], []
        self.box("floor", (0, 0, -0.05), (L, W, 0.1), floor, collide=False)
        self.box("ceiling", (0, 0, H + 0.05), (L, W, 0.1), "L5_245_242_236", collide=False)
        t = 0.2
        self.box("wall_back", (L / 2 + t / 2, 0, H / 2), (t, W + 2 * t, H), wall, collide=False)
        self.box("wall_l", (0, W / 2 + t / 2, H / 2), (L, t, H), wall, collide=False)
        self.box("wall_r", (0, -W / 2 - t / 2, H / 2), (L, t, H), wall, collide=False)
        seg = (W - door_w) / 2
        self.box("wall_front_a", (-L / 2 - t / 2, door_w / 2 + seg / 2, H / 2), (t, seg, H), wall, collide=False)
        self.box("wall_front_b", (-L / 2 - t / 2, -door_w / 2 - seg / 2, H / 2), (t, seg, H), wall, collide=False)
        self.box("lintel", (-L / 2 - t / 2, 0, (H + 2.3) / 2), (t, door_w, H - 2.3), wall, collide=False)
        self.box("door_light", (-L / 2 - t, 0, 1.15), (0.02, door_w, 2.3), "EMIT_255_240_210", collide=False)
        # skirting boards
        for y in (W / 2 - 0.01, -W / 2 + 0.01):
            self.box("skirting", (0, y, 0.06), (L, 0.02, 0.12), WOOD_DARK, collide=False)
        self.spots.append({"name": "sortie", "label": "E : sortir", "x": -L / 2 + 0.8, "z": 0.0, "yaw": math.pi})

    def box(self, name, c, s, material, bevel=0.0, collide=True, rot=0.0):
        bpy.ops.mesh.primitive_cube_add(size=1, location=c, rotation=(0, 0, rot))
        o = bpy.context.active_object
        o.name = name
        o.scale = s
        bpy.ops.object.transform_apply(scale=True)
        o.data.materials.append(mat(material))
        if bevel > 0:
            b = o.modifiers.new("b", "BEVEL")
            b.width = bevel
            b.segments = 3
        self.parts.append(o)
        if collide and c[2] - s[2] / 2 < 1.2:
            hx, hy = s[0] / 2, s[1] / 2
            if rot:
                r = max(hx, hy)
                hx = hy = r
            self.colliders.append([c[0] - hx, -(c[1] + hy), c[0] + hx, -(c[1] - hy)])
        return o

    def cyl(self, name, c, r, h, material, verts=20, collide=True, r2=None, rot=None):
        bpy.ops.mesh.primitive_cone_add(radius1=r, radius2=r if r2 is None else r2, depth=h, location=c, vertices=verts, rotation=rot or (0, 0, 0))
        o = bpy.context.active_object
        o.name = name
        o.data.materials.append(mat(material))
        for p in o.data.polygons:
            p.use_smooth = True
        self.parts.append(o)
        if collide and c[2] - h / 2 < 1.2:
            self.colliders.append([c[0] - r, -(c[1] + r), c[0] + r, -(c[1] - r)])
        return o

    def sphere(self, name, c, r, material, scale=(1, 1, 1)):
        bpy.ops.mesh.primitive_uv_sphere_add(radius=r, location=c, segments=16, ring_count=10)
        o = bpy.context.active_object
        o.scale = scale
        bpy.ops.object.transform_apply(scale=True)
        o.data.materials.append(mat(material))
        for p in o.data.polygons:
            p.use_smooth = True
        self.parts.append(o)
        return o

    def spot(self, name, label, x, y, yaw):
        # yaw in game space (angle of the facing direction in the x/z plane, z = -y)
        self.spots.append({"name": name, "label": label, "x": x, "z": -y, "yaw": yaw})

    def lamp(self, x, y):
        self.cyl("lamp_cord", (x, y, self.H - 0.25), 0.01, 0.5, BLACK, 6, False)
        self.cyl("lamp_shade", (x, y, self.H - 0.55), 0.28, 0.25, "L8_235_200_120", 24, False, 0.12)
        self.sphere("bulb", (x, y, self.H - 0.66), 0.09, LAMP)

    def window(self, x, side, w=1.6, h=1.3, z=1.5):
        y = side * (self.W / 2 - 0.03)
        self.box("win", (x, y, z), (w, 0.02, h), WINDOW, collide=False)
        for dz in (-h / 2, h / 2):
            self.box("frame", (x, y - side * 0.02, z + dz), (w + 0.1, 0.06, 0.06), "L8_240_240_240", collide=False)
        for dx in (-w / 2, 0, w / 2):
            self.box("frame", (x + dx, y - side * 0.02, z), (0.06, 0.06, h), "L8_240_240_240", collide=False)

    def plant(self, x, y, s=1.0):
        self.cyl("pot", (x, y, 0.25 * s), 0.25 * s, 0.5 * s, "L8_170_90_55", 16, True, 0.2 * s)
        for k in range(6):
            a = k / 6 * math.tau
            self.sphere("leaf", (x + 0.15 * s * math.cos(a), y + 0.15 * s * math.sin(a), 0.75 * s + 0.15 * s * (k % 2)), 0.22 * s, "L9_210_230_190", (1, 1, 1.6))

    def chair(self, x, y, yaw, color):
        # moulded plastic chair as in every maquis
        c, s = math.cos(yaw), math.sin(yaw)
        self.box("seat", (x, y, 0.45), (0.45, 0.45, 0.05), color, 0.02, False, yaw)
        for dx, dy in ((0.19, 0.19), (0.19, -0.19), (-0.19, 0.19), (-0.19, -0.19)):
            self.cyl("leg", (x + dx * c - dy * s, y + dx * s + dy * c, 0.22), 0.02, 0.44, color, 8, False)
        bx, by = x - 0.22 * c, y - 0.22 * s
        self.box("back", (bx, by, 0.75), (0.05, 0.45, 0.55), color, 0.02, False, yaw)
        self.colliders.append([x - 0.25, -(y + 0.25), x + 0.25, -(y - 0.25)])

    def finish(self):
        export(self.parts, os.path.join(OUT, self.name + ".ocs"), uv_scale=1.5)
        data = {"size": [self.L, self.W, self.H], "colliders": self.colliders, "spots": self.spots}
        json.dump(data, open(os.path.join(OUT, self.name + ".json"), "w"), ensure_ascii=False)
        print("room", self.name, len(self.colliders), "colliders", len(self.spots), "spots")


def maquis():
    r = Room("maquis", 14.0, 10.0, 3.4, TILES, "L5_236_196_146")
    L, W, H = r.L, r.W, r.H
    # wood panelling at the bottom of the walls
    for y, sg in ((W / 2 - 0.03, 1), (-W / 2 + 0.03, -1)):
        r.box("panel", (0, y, 0.55), (L, 0.04, 1.1), WOOD, collide=False)
    # bar counter with a marble top
    cx = L / 2 - 1.6
    r.box("counter", (cx, 0, 0.55), (0.8, 5.5, 1.1), WOOD_DARK, 0.03)
    r.box("counter_top", (cx, 0, 1.13), (0.95, 5.7, 0.06), "L17_200_200_200", 0.01, False)
    for k in range(5):
        r.cyl("stool", (cx - 0.75, -2.0 + k, 0.38), 0.18, 0.06, "L8_200_40_40", 16, False)
        r.cyl("stool_leg", (cx - 0.75, -2.0 + k, 0.18), 0.03, 0.36, CHROME, 8, False)
    # back shelves with bottles, fridge, music
    r.box("shelf", (L / 2 - 0.25, 0, 1.6), (0.4, 5.0, 0.05), WOOD, collide=False)
    r.box("shelf2", (L / 2 - 0.25, 0, 2.1), (0.4, 5.0, 0.05), WOOD, collide=False)
    for k in range(26):
        y = -2.4 + k * 0.19
        z = 1.75 if k % 2 == 0 else 2.25
        col = random.choice(["L6_40_110_50", "L6_120_60_20", "L6_200_200_190", "L6_150_30_30", "L6_230_170_40"])
        r.cyl("bottle", (L / 2 - 0.25, y, z), 0.04, 0.28, col, 10, False, 0.025)
    r.box("fridge", (L / 2 - 0.45, 3.9, 1.0), (0.7, 1.2, 2.0), "L8_230_230_235", 0.03)
    r.box("fridge_glass", (L / 2 - 0.81, 3.9, 1.1), (0.02, 1.0, 1.6), "EMIT_190_225_255", collide=False)
    r.box("speaker", (L / 2 - 0.3, -3.9, 0.6), (0.5, 0.5, 1.2), BLACK, 0.03)
    # TV with a football match on the side wall
    r.box("tv", (1.0, W / 2 - 0.08, 2.1), (1.8, 0.08, 1.05), BLACK, 0.02, False)
    r.box("tv_screen", (1.0, W / 2 - 0.135, 2.1), (1.65, 0.01, 0.92), "EMIT_60_150_70", collide=False)
    # tables and plastic chairs
    colors = ["L8_210_40_40", "L8_30_90_190", "L8_240_240_235", "L8_250_200_40"]
    k = 0
    for tx in (-4.6, -1.6, 1.4):
        for ty in (-2.6, 2.2):
            r.cyl("table_top", (tx, ty, 0.74), 0.55, 0.04, "L19_230_210_190", 28, False)
            r.cyl("table_leg", (tx, ty, 0.37), 0.05, 0.72, METAL, 10, True)
            r.cyl("table_foot", (tx, ty, 0.02), 0.28, 0.04, METAL, 16, False)
            # a plate, a glass and a bottle of local drink on each table
            r.cyl("plate", (tx + 0.15, ty, 0.77), 0.14, 0.02, "L8_245_245_240", 20, False)
            r.cyl("glass", (tx - 0.2, ty + 0.15, 0.82), 0.04, 0.12, "L6_220_230_235", 10, False)
            r.cyl("bottle", (tx - 0.1, ty - 0.2, 0.89), 0.035, 0.26, "L6_40_110_50", 10, False, 0.022)
            col = colors[k % 4]
            for a in range(4):
                ang = a * math.pi / 2 + 0.3
                x, y = tx + math.cos(ang) * 0.9, ty + math.sin(ang) * 0.9
                r.chair(x, y, ang + math.pi, col)
                # sitting spot: facing the table (game yaw: direction in x/z, z = -y)
                fx, fy = -math.cos(ang), -math.sin(ang)
                r.spot(f"table{k}_{a}", "E : s'asseoir et manger (1 500 FCFA)", x, y, math.atan2(-fy, fx))
            k += 1
    for x in (-4.6, -1.6, 1.4, 4.0):
        r.lamp(x, 0)
    for x in (-4.5, -0.5, 3.5):
        r.window(x, 1)
        r.window(x, -1)
    # wax curtains by the door
    for y in (0.95, -0.95):
        r.box("curtain", (-L / 2 + 0.06, y, 1.3), (0.04, 0.5, 2.4), "L14_255_255_255", collide=False)
    r.plant(-L / 2 + 0.6, W / 2 - 0.6)
    r.plant(-L / 2 + 0.6, -W / 2 + 0.6)
    r.plant(L / 2 - 0.6, -W / 2 + 1.4, 0.8)
    # menu board
    r.box("menu", (-2.0, -W / 2 + 0.06, 2.0), (2.2, 0.04, 1.0), "L8_30_40_30", 0.02, False)
    for k in range(5):
        r.box("menu_line", (-2.0, -W / 2 + 0.085, 2.3 - k * 0.15), (1.6 - (k % 2) * 0.4, 0.01, 0.04), "L8_240_240_230", collide=False)
    r.spot("comptoir", "E : commander à manger (1 500 FCFA)", cx - 0.9, 0.0, 0.0)
    r.finish()


def appartement():
    r = Room("appartement", 12.0, 9.0, 3.0, PARQUET, "L5_242_236_226")
    L, W, H = r.L, r.W, r.H
    # partition between living room and bedroom, with an opening
    r.box("partition_a", (2.0, 2.4, H / 2), (0.15, 4.2, H), "L5_242_236_226", collide=True)
    r.box("partition_b", (2.0, -3.3, H / 2), (0.15, 2.4, H), "L5_242_236_226", collide=True)
    # bedroom: double bed with a wax duvet, nightstands, wardrobe
    bx = 4.3
    r.box("bed_frame", (bx, 1.8, 0.25), (2.1, 1.9, 0.5), WOOD_DARK, 0.03)
    r.box("mattress", (bx, 1.8, 0.58), (2.0, 1.8, 0.2), "L18_250_250_250", 0.06, False)
    r.box("duvet", (bx - 0.2, 1.8, 0.7), (1.5, 1.84, 0.08), "L15_255_255_255", 0.04, False)
    for y in (1.35, 2.25):
        r.box("pillow", (bx + 0.82, y, 0.75), (0.3, 0.7, 0.16), "L18_255_255_255", 0.07, False)
    r.box("headboard", (bx + 1.1, 1.8, 0.85), (0.1, 2.0, 1.2), WOOD_DARK, 0.03)
    for y in (0.55, 3.05):
        r.box("nightstand", (bx + 0.85, y, 0.3), (0.45, 0.45, 0.6), WOOD, 0.02)
    r.cyl("lamp_base", (bx + 0.85, 3.05, 0.75), 0.08, 0.3, "L8_200_170_110", 12, False)
    r.cyl("lamp_shade", (bx + 0.85, 3.05, 1.0), 0.18, 0.22, "EMIT_255_225_170", 16, False, 0.12)
    r.box("wardrobe", (bx, -3.85, 1.1), (2.0, 0.65, 2.2), WOOD, 0.02)
    r.spot("lit", "E : dormir jusqu'au matin (sauvegarde)", bx - 1.2, 1.8, 0.0)
    r.spot("armoire", "E : ouvrir l'armoire et changer de tenue", bx, -3.0, -math.pi / 2)
    # living room: sofa, coffee table, TV unit, rug
    r.box("rug", (-2.0, 0.6, 0.01), (3.2, 2.4, 0.02), "L14_255_255_255", collide=False)
    r.box("sofa_base", (-2.0, 2.9, 0.25), (2.6, 0.95, 0.45), "L18_70_95_140", 0.08)
    r.box("sofa_back", (-2.0, 3.3, 0.7), (2.6, 0.25, 0.65), "L18_70_95_140", 0.08, False)
    for x in (-3.35, -0.65):
        r.box("sofa_arm", (x, 2.9, 0.55), (0.25, 0.95, 0.35), "L18_70_95_140", 0.08, False)
    for x in (-2.6, -1.4):
        r.box("cushion", (x, 3.05, 0.75), (0.45, 0.15, 0.4), "L15_255_255_255", 0.05, False)
    r.box("coffee_table", (-2.0, 0.9, 0.22), (1.3, 0.7, 0.44), WOOD, 0.02)
    r.box("tv_unit", (-2.0, -1.4, 0.3), (2.2, 0.5, 0.6), WOOD_DARK, 0.02)
    r.box("tv", (-2.0, -1.45, 1.05), (1.6, 0.06, 0.92), BLACK, 0.02, False)
    r.box("tv_screen", (-2.0, -1.41, 1.05), (1.5, 0.01, 0.84), "EMIT_70_110_160", collide=False)
    r.spot("canape", "E : s'asseoir devant la télé", -2.0, 2.6, -math.pi / 2 * -1 if False else math.pi / 2)
    # kitchen corner
    r.box("kitchen", (-4.9, -3.6, 0.45), (2.6, 0.65, 0.9), "L8_235_235_232", 0.02)
    r.box("worktop", (-4.9, -3.6, 0.92), (2.7, 0.7, 0.05), "L17_180_180_180", 0.01, False)
    r.box("stove", (-5.6, -3.6, 0.96), (0.6, 0.55, 0.03), BLACK, collide=False)
    r.cyl("pot", (-5.6, -3.6, 1.05), 0.14, 0.18, CHROME, 16, False)
    r.box("fridge", (-3.25, -3.65, 0.95), (0.7, 0.65, 1.9), "L8_240_240_240", 0.04)
    r.spot("frigo", "E : manger un morceau", -3.25, -2.9, -math.pi / 2)
    # dining table
    r.box("dining", (-4.6, 0.4, 0.74), (1.4, 0.9, 0.05), WOOD, 0.02, False)
    r.box("dining_leg", (-4.6, 0.4, 0.36), (1.2, 0.7, 0.7), WOOD_DARK, 0.0, True)
    for x, y, yaw in ((-4.6, 1.15, -math.pi / 2), (-4.6, -0.35, math.pi / 2)):
        r.chair(x, y, yaw, WOOD)
    for x in (-2.5, 4.0):
        r.lamp(x, 0.5)
    r.window(-2.0, 1, 2.0)
    r.window(4.3, -1, 1.4)
    r.window(-4.6, 1, 1.2)
    r.plant(-5.4, 3.8)
    r.plant(1.4, -3.9, 0.8)
    # framed photos
    for k, x in enumerate((-3.0, -2.0, -1.0)):
        r.box("frame", (x, W / 2 - 0.04, 1.8), (0.6, 0.03, 0.45), WOOD_DARK, 0.01, False)
        r.box("photo", (x, W / 2 - 0.06, 1.8), (0.5, 0.01, 0.35), ["L14_255_255_255", "L8_200_150_90", "L15_255_255_255"][k], collide=False)
    r.finish()


def salle_sport():
    r = Room("salle_sport", 16.0, 12.0, 4.0, "L5_52_52_56", "L5_225_225_228")
    L, W, H = r.L, r.W, r.H
    r.box("mirror", (L / 2 - 0.04, 0, 1.6), (0.03, W - 1.0, 2.4), "L6_225_232_240", collide=False)
    # benches with barbells
    for k, y in enumerate((-3.5, 0.0, 3.5)):
        x = 3.5
        r.box("bench", (x, y, 0.42), (1.2, 0.35, 0.1), "L18_30_30_35", 0.03)
        r.box("bench_leg", (x, y, 0.2), (1.0, 0.25, 0.4), METAL, 0.0, False)
        for dy in (-0.55, 0.55):
            r.cyl("rack", (x + 0.55, y + dy, 0.6), 0.04, 1.2, METAL, 8, False)
        r.cyl("bar", (x + 0.55, y, 1.15), 0.025, 1.8, CHROME, 8, False, None, (math.pi / 2, 0, 0))
        for dy in (-0.8, 0.8):
            r.cyl("plate", (x + 0.55, y + dy, 1.15), 0.22, 0.06, BLACK, 20, False, None, (math.pi / 2, 0, 0))
    # dumbbell rack
    r.box("db_rack", (L / 2 - 0.6, 0, 0.45), (0.6, 5.0, 0.12), METAL, 0.02)
    for k in range(10):
        y = -2.2 + k * 0.5
        for dy in (-0.13, 0.13):
            r.cyl("db", (L / 2 - 0.6, y + dy, 0.58), 0.07 + k * 0.004, 0.08, BLACK, 12, False, None, (math.pi / 2, 0, 0))
        r.cyl("db_bar", (L / 2 - 0.6, y, 0.58), 0.015, 0.3, CHROME, 6, False, None, (math.pi / 2, 0, 0))
    # treadmills
    for y in (-4.2, -2.6):
        r.box("tread_base", (-2.0, y, 0.15), (1.9, 0.8, 0.3), BLACK, 0.04)
        r.box("tread_belt", (-2.0, y, 0.31), (1.7, 0.6, 0.02), "L8_40_40_42", collide=False)
        r.box("tread_console", (-1.0, y, 1.25), (0.15, 0.7, 0.35), "L8_60_62_70", 0.03, False)
        for dy in (-0.35, 0.35):
            r.cyl("tread_post", (-1.05, y + dy, 0.7), 0.03, 1.2, METAL, 8, False)
        r.spot(f"tapis{int(y)}", "E : courir sur le tapis (endurance)", -2.3, y, 0.0)
    # punching bag on a chain
    r.cyl("bag_chain", (-4.0, 3.0, H - 0.6), 0.01, 1.2, CHROME, 6, False)
    r.cyl("bag", (-4.0, 3.0, 1.5), 0.25, 1.1, "L18_150_25_25", 20, True)
    r.spot("sac", "E : boxer le sac de frappe (force)", -4.75, 3.0, 0.0)
    # mats for squats
    for k, y in enumerate((-1.5, 0.5)):
        r.box("mat", (-5.0, y, 0.015), (1.8, 0.9, 0.03), "L18_40_100_170", collide=False)
        r.spot(f"squat{k}", "E : faire des squats (forme)", -5.0, y, 0.0)
    for x in (-4.0, 0.0, 4.0):
        for y in (-3.0, 3.0):
            r.box("light", (x, y, H - 0.04), (2.0, 0.25, 0.05), LAMP, collide=False)
    for x in (-5.0, -1.0, 3.0):
        r.window(x, 1, 2.2, 1.0, 2.6)
    r.plant(-L / 2 + 0.6, -W / 2 + 0.6)
    r.box("water", (-L / 2 + 0.6, W / 2 - 0.6, 0.6), (0.4, 0.4, 1.2), "L8_235_235_240", 0.03)
    r.cyl("bottle", (-L / 2 + 0.6, W / 2 - 0.6, 1.4), 0.15, 0.4, "L6_150_200_240", 16, False)
    r.finish()


def garage():
    r = Room("garage", 10.0, 6.5, 3.4, "L5_150_150_150", "L5_205_200_190", 3.2)
    L, W, H = r.L, r.W, r.H
    # rolled-up shutter above the door
    r.cyl("shutter", (-L / 2 + 0.3, 0, H - 0.3), 0.25, 3.3, "L4_220_220_225", 16, False, None, (math.pi / 2, 0, 0))
    r.box("bench", (L / 2 - 0.5, -1.0, 0.45), (0.8, 3.0, 0.9), WOOD_DARK, 0.02)
    r.box("bench_top", (L / 2 - 0.5, -1.0, 0.92), (0.85, 3.1, 0.05), WOOD, 0.01, False)
    r.box("tool_board", (L / 2 - 0.05, -1.0, 1.8), (0.04, 2.8, 1.2), "L19_200_170_130", collide=False)
    for k in range(9):
        r.box("tool", (L / 2 - 0.09, -2.1 + k * 0.28, 1.8 + (k % 3 - 1) * 0.3), (0.03, 0.05, 0.4), random.choice([METAL, "L8_200_40_40", "L8_230_180_30"]), collide=False)
    for k in range(3):
        r.cyl("tyre", (L / 2 - 0.6, W / 2 - 0.6, 0.12 + k * 0.24), 0.33, 0.22, BLACK, 24, k == 0)
    for k, y in enumerate((-W / 2 + 0.5, -W / 2 + 1.2)):
        r.cyl("barrel", (2.0, y, 0.45), 0.3, 0.9, ["L8_30_90_170", "L8_200_60_40"][k], 20, True)
    r.box("oil_stain", (0.0, 0.0, 0.006), (2.0, 1.2, 0.01), "L8_70_70_72", collide=False)
    for x in (-2.0, 2.0):
        r.box("light", (x, 0, H - 0.04), (1.8, 0.2, 0.05), LAMP, collide=False)
    r.spot("vehicule", "E : sortir ton véhicule du garage", -2.0, 0.0, math.pi)
    r.finish()


def hopital():
    r = Room("hopital", 16.0, 11.0, 3.4, "L17_235_240_240", "L5_236_242_240")
    L, W, H = r.L, r.W, r.H
    # reception desk
    r.box("desk", (-3.0, 0.0, 0.55), (1.0, 3.6, 1.1), "L8_240_240_240", 0.03)
    r.box("desk_top", (-3.0, 0.0, 1.12), (1.15, 3.8, 0.05), "L8_30_120_120", 0.01, False)
    r.box("screen", (-2.8, 0.8, 1.35), (0.05, 0.5, 0.35), BLACK, 0.01, False)
    r.box("sign", (-2.95, 0.0, 2.6), (0.04, 2.4, 0.5), "L8_30_120_120", collide=False)
    r.box("cross_v", (-3.0, 0.0, 2.6), (0.02, 0.16, 0.42), "EMIT_240_60_60", collide=False)
    r.box("cross_h", (-3.0, 0.0, 2.6), (0.02, 0.42, 0.16), "EMIT_240_60_60", collide=False)
    # waiting chairs
    for k in range(6):
        r.chair(-5.8, -4.2 + k * 0.6, 0.0, "L8_40_110_170")
    # beds with curtains
    for k, y in enumerate((-3.6, -1.2, 1.2, 3.6)):
        x = 4.6
        r.box("bed", (x, y, 0.4), (2.0, 0.95, 0.15), "L18_250_250_250", 0.03)
        r.box("bed_frame", (x, y, 0.22), (2.0, 0.95, 0.3), CHROME, 0.0)
        r.box("pillow", (x + 0.8, y, 0.52), (0.3, 0.7, 0.12), "L18_255_255_255", 0.05, False)
        r.box("sheet", (x - 0.25, y, 0.5), (1.4, 0.98, 0.04), "L18_150_200_230", 0.02, False)
        r.cyl("drip", (x + 1.2, y + 0.55, 0.9), 0.02, 1.8, CHROME, 6, False)
        r.box("drip_bag", (x + 1.2, y + 0.55, 1.75), (0.05, 0.12, 0.2), "L6_220_235_240", collide=False)
        r.box("curtain", (x - 0.2, y + 0.62, 1.3), (2.4, 0.03, 2.0), "L18_160_210_200", collide=False)
    for x in (-4.0, 0.0, 4.0):
        r.box("light", (x, 0, H - 0.04), (2.4, 0.4, 0.05), LAMP, collide=False)
    for x in (-5.0, 0.0):
        r.window(x, 1, 1.8)
    r.plant(-L / 2 + 0.6, W / 2 - 0.6)
    r.box("cabinet", (L / 2 - 0.4, 0, 1.0), (0.6, 2.0, 2.0), "L8_240_240_240", 0.02)
    r.spot("accueil", "E : se faire soigner (santé au max, 2 000 FCFA)", -3.9, 0.0, 0.0)
    r.spot("lit_hop", "E : se reposer sur un lit (santé +25)", 3.4, -1.2, 0.0)
    r.finish()


def boutique():
    r = Room("boutique", 12.0, 9.0, 3.2, "L17_230_225_215", "L5_245_238_225")
    L, W, H = r.L, r.W, r.H
    # shelves full of goods
    goods = ["L6_200_40_40", "L6_240_200_40", "L6_40_120_200", "L6_60_160_70", "L6_240_240_235", "L6_230_120_40"]
    for k, y in enumerate((-2.4, 0.0, 2.4)):
        r.box("shelf", (0.5, y, 0.9), (5.0, 0.6, 1.8), WOOD, 0.01)
        for lvl in range(4):
            for j in range(18):
                col = goods[(j * 7 + lvl * 3 + k) % len(goods)]
                for side in (-1, 1):
                    r.box("good", (-1.8 + j * 0.27, y + side * 0.33, 0.25 + lvl * 0.45), (0.2, 0.06, 0.3), col, collide=False)
    # counter with a cash register
    r.box("counter", (-3.6, -2.6, 0.5), (1.6, 0.7, 1.0), WOOD_DARK, 0.02)
    r.box("register", (-3.6, -2.6, 1.12), (0.4, 0.35, 0.22), BLACK, 0.02, False)
    r.box("fridge", (L / 2 - 0.45, 3.4, 1.0), (0.7, 1.6, 2.0), "L8_230_230_235", 0.03)
    r.box("fridge_glass", (L / 2 - 0.81, 3.4, 1.1), (0.02, 1.4, 1.6), "EMIT_190_225_255", collide=False)
    # clothes rail and fitting booth
    r.box("rail", (L / 2 - 0.8, -2.0, 1.7), (0.05, 2.6, 0.05), CHROME, collide=False)
    for k in range(10):
        r.box("cloth", (L / 2 - 0.8, -3.1 + k * 0.25, 1.25), (0.35, 0.05, 0.8), ["L14_255_255_255", "L26_255_255_255", "L27_255_255_255", "L28_255_255_255", "L15_255_255_255"][k % 5], collide=False)
    r.box("booth", (L / 2 - 0.6, 1.0, 1.1), (1.1, 0.06, 2.2), "L14_255_255_255", collide=False)
    r.box("mirror", (L / 2 - 0.04, 0.4, 1.2), (0.03, 0.8, 1.8), "L6_225_232_240", collide=False)
    for x in (-3.0, 1.0, 4.0):
        r.box("light", (x, 0, H - 0.04), (1.8, 0.3, 0.05), LAMP, collide=False)
    r.window(-3.0, 1, 2.0)
    r.spot("caisse", "E : acheter à manger et à boire (500 FCFA)", -3.6, -1.85, -math.pi / 2)
    r.spot("cabine", "E : essayer une tenue (cabine d'essayage)", L / 2 - 1.0, 0.4, 0.0)
    r.finish()


def pharmacie():
    r = Room("pharmacie", 10.0, 8.0, 3.2, "L17_240_245_240", "L5_240_246_240")
    L, W, H = r.L, r.W, r.H
    r.box("counter", (1.5, 0.0, 0.55), (0.8, 5.0, 1.1), "L8_245_245_245", 0.02)
    r.box("counter_top", (1.5, 0.0, 1.12), (0.9, 5.2, 0.05), "L8_40_160_80", 0.01, False)
    r.box("back_shelf", (L / 2 - 0.3, 0, 1.3), (0.5, 6.5, 2.6), "L8_245_245_245", 0.01)
    for lvl in range(5):
        for j in range(24):
            col = ["L6_240_240_240", "L6_40_160_80", "L6_200_60_60", "L6_60_120_200"][(j + lvl) % 4]
            r.box("box", (L / 2 - 0.58, -3.0 + j * 0.26, 0.35 + lvl * 0.45), (0.04, 0.2, 0.3), col, collide=False)
    r.box("cross_v", (-L / 2 + 0.06, 0, 2.5), (0.02, 0.2, 0.6), "EMIT_60_220_90", collide=False)
    r.box("cross_h", (-L / 2 + 0.06, 0, 2.5), (0.02, 0.6, 0.2), "EMIT_60_220_90", collide=False)
    for x in (-2.0, 2.0):
        r.box("light", (x, 0, H - 0.04), (1.6, 0.3, 0.05), LAMP, collide=False)
    r.plant(-L / 2 + 0.6, W / 2 - 0.6)
    r.spot("pharma", "E : acheter des médicaments (santé +40, 800 FCFA)", 0.8, 0.0, 0.0)
    r.finish()


def banque():
    r = Room("banque", 14.0, 10.0, 3.8, "L39_255_255_255", "L5_238_234_226")
    L, W, H = r.L, r.W, r.H
    # counters behind glass
    r.box("counter", (3.0, 0.0, 0.6), (0.9, 7.0, 1.2), WOOD_DARK, 0.02)
    r.box("counter_top", (3.0, 0.0, 1.22), (1.0, 7.2, 0.05), "L17_200_200_200", 0.01, False)
    r.box("glass", (3.0, 0.0, 1.9), (0.03, 7.0, 1.3), "L6_200_225_235", collide=False)
    for y in (-2.4, 0.0, 2.4):
        r.box("screen", (3.3, y, 1.45), (0.05, 0.5, 0.35), BLACK, 0.01, False)
    # ATM on the side wall
    r.box("atm", (-3.0, -W / 2 + 0.4, 0.9), (0.8, 0.6, 1.8), "L8_60_70_90", 0.03)
    r.box("atm_screen", (-3.0, -W / 2 + 0.71, 1.25), (0.4, 0.02, 0.3), "EMIT_120_200_255", collide=False)
    # waiting sofa and plants
    r.box("sofa", (-3.0, W / 2 - 0.6, 0.3), (2.6, 0.8, 0.5), "L18_120_30_30", 0.06)
    r.plant(-L / 2 + 0.6, W / 2 - 0.6)
    r.plant(-L / 2 + 0.6, -W / 2 + 0.6)
    for x in (-3.0, 1.0):
        r.lamp(x, 0)
    r.window(-3.0, 1, 2.2)
    r.spot("guichet", "E : déposer ton argent liquide à la banque", 2.2, 0.0, 0.0)
    r.spot("dab", "E : retirer 10 000 FCFA au distributeur", -3.0, -W / 2 + 1.1, -math.pi / 2)
    r.finish()


maquis()
appartement()
salle_sport()
garage()
hopital()
boutique()
pharmacie()
banque()
