# Blender script: model the cars of Ouaga City (sedan, green taxi, police car, minibus) and a wheel.
# Usage: blender -b -P build_vehicles.py -- <out_dir>
import bpy, bmesh, sys, os, math
from mathutils import Vector

sys.path.append(os.path.dirname(__file__))
from ocs_export import export, mat

OUT = sys.argv[sys.argv.index("--") + 1]


def clear():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def profile_body(name, prof, width, cabin_from, cabin_scale, bevel=0.07):
    """Side profile (x along the length, z up) extruded across the width, cabin narrowed, bevelled."""
    me = bpy.data.meshes.new(name)
    bm = bmesh.new()
    left = [bm.verts.new((x, width / 2, z)) for x, z in prof]
    right = [bm.verts.new((x, -width / 2, z)) for x, z in prof]
    bm.faces.new(left)
    bm.faces.new(list(reversed(right)))
    n = len(prof)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new([left[i], left[j], right[j], right[i]])
    for v in bm.verts:
        if v.co.z > cabin_from:
            k = min(1.0, (v.co.z - cabin_from) / 0.25)
            v.co.y *= 1.0 - (1.0 - cabin_scale) * k
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(me)
    bm.free()
    o = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(o)
    bev = o.modifiers.new("bevel", "BEVEL")
    bev.width = bevel
    bev.segments = 3
    bev.limit_method = "ANGLE"
    return o


def cut_arches(body, xs, r, width):
    for x in xs:
        bpy.ops.mesh.primitive_cylinder_add(radius=r, depth=width + 0.4, location=(x, 0, r * 0.95), rotation=(math.pi / 2, 0, 0), vertices=24)
        cyl = bpy.context.active_object
        cyl.hide_render = True
        b = body.modifiers.new("arch", "BOOLEAN")
        b.operation = "DIFFERENCE"
        b.object = cyl
        b.solver = "EXACT"
        cyl.display_type = "WIRE"
    # boolean before the bevel
    body.modifiers.move(len(body.modifiers) - 1, 0) if False else None


def assign(body, rules):
    """rules: list of (material name, predicate(face centre, normal)) — first match wins; default paint."""
    me = body.data
    names = ["PAINT"] + [r[0] for r in rules]
    body.data.materials.clear()
    for n in names:
        body.data.materials.append(mat(n))
    for p in me.polygons:
        c, n = p.center, p.normal
        p.material_index = 0
        for k, (name, pred) in enumerate(rules):
            if pred(c, n):
                p.material_index = k + 1
                break


def box(name, loc, size, material, bevel=0.0):
    bpy.ops.mesh.primitive_cube_add(size=1, location=loc)
    o = bpy.context.active_object
    o.name = name
    o.scale = size
    bpy.ops.object.transform_apply(scale=True)
    o.data.materials.append(mat(material))
    if bevel > 0:
        b = o.modifiers.new("b", "BEVEL")
        b.width = bevel
        b.segments = 2
    return o


def cyl(name, loc, r, depth, material, rot=(0, 0, 0), verts=20):
    bpy.ops.mesh.primitive_cylinder_add(radius=r, depth=depth, location=loc, rotation=rot, vertices=verts)
    o = bpy.context.active_object
    o.name = name
    o.data.materials.append(mat(material))
    return o


GLASS = "L6_40_52_64"
TRIM = "L8_30_30_33"
CHROME = "L8_190_192_198"
HEAD = "EMIT_255_248_225"
TAIL = "EMIT_210_25_25"


def body_mesh(name, prof, width, cabin_from, cabin_scale, cuts_z, cuts_x):
    me = bpy.data.meshes.new(name)
    bm = bmesh.new()
    left = [bm.verts.new((x, width / 2, z)) for x, z in prof]
    right = [bm.verts.new((x, -width / 2, z)) for x, z in prof]
    bm.faces.new(left)
    bm.faces.new(list(reversed(right)))
    n = len(prof)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new([left[i], left[j], right[j], right[i]])
    for z in cuts_z:
        geom = bm.verts[:] + bm.edges[:] + bm.faces[:]
        bmesh.ops.bisect_plane(bm, geom=geom, plane_co=(0, 0, z), plane_no=(0, 0, 1))
    for x in cuts_x:
        geom = bm.verts[:] + bm.edges[:] + bm.faces[:]
        bmesh.ops.bisect_plane(bm, geom=geom, plane_co=(x, 0, 0), plane_no=(1, 0, 0))
    for v in bm.verts:
        if v.co.z > cabin_from:
            k = min(1.0, (v.co.z - cabin_from) / 0.3)
            v.co.y *= 1.0 - (1.0 - cabin_scale) * k
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(me)
    bm.free()
    o = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(o)
    return o


def finish(body, rules, arches, r, width):
    assign(body, rules)
    bev = body.modifiers.new("bevel", "BEVEL")
    bev.width = 0.045
    bev.segments = 2
    bev.limit_method = "ANGLE"
    sub = body.modifiers.new("sub", "SUBSURF")
    sub.levels = 1
    sub.render_levels = 1
    for x in arches:
        bpy.ops.mesh.primitive_cylinder_add(radius=r, depth=width + 0.5, location=(x, 0, r * 0.82), rotation=(math.pi / 2, 0, 0), vertices=32)
        c = bpy.context.active_object
        b = body.modifiers.new("arch", "BOOLEAN")
        b.operation = "DIFFERENCE"
        b.object = c
        b.solver = "EXACT"
        c.hide_set(True)
    bpy.context.view_layer.objects.active = body
    for m in list(body.modifiers):
        bpy.ops.object.modifier_apply(modifier=m.name)
    for o in list(bpy.data.objects):
        if o.type == "MESH" and o.name.startswith("Cylinder"):
            bpy.data.objects.remove(o)
    body.data.shade_smooth() if hasattr(body.data, "shade_smooth") else None
    for p in body.data.polygons:
        p.use_smooth = True


def car(kind):
    clear()
    W = 1.82
    prof = [(-2.18, 0.3), (2.12, 0.3), (2.24, 0.42), (2.26, 0.6), (2.2, 0.74), (1.9, 0.82), (1.15, 0.88), (0.95, 0.95),
            (0.5, 1.34), (0.3, 1.4), (-0.85, 1.42), (-1.1, 1.36), (-1.65, 0.98), (-2.1, 0.94), (-2.24, 0.8), (-2.26, 0.5)]
    body = body_mesh("body", prof, W, 0.9, 0.84, [0.45, 0.9], [-1.62, -0.5, -0.35, 0.92])
    win = lambda c, n: c.z > 0.92 and n.z < 0.8 and -1.62 < c.x < 0.95 and not (-0.5 < c.x < -0.35 and abs(n.y) > 0.5)
    rules = [(GLASS, win), (TRIM, lambda c, n: c.z < 0.45)]
    if kind == "police":
        rules.insert(1, ("L8_238_238_238", lambda c, n: abs(n.y) > 0.6 and 0.45 < c.z < 0.9 and -1.62 < c.x < 0.92))
    finish(body, rules, (1.35, -1.35), 0.42, W)
    parts = [body]
    for y in (0.55, -0.55):
        parts.append(box("hl", (2.21, y, 0.64), (0.07, 0.36, 0.1), HEAD, 0.03))
        parts.append(box("tl", (-2.23, y, 0.72), (0.07, 0.32, 0.1), TAIL, 0.03))
    parts.append(box("grille", (2.25, 0, 0.5), (0.04, 0.72, 0.12), TRIM, 0.01))
    parts.append(box("plate", (-2.27, 0, 0.5), (0.02, 0.5, 0.12), "L8_235_235_230"))
    for y in (W / 2 + 0.03, -W / 2 - 0.03):
        parts.append(box("mirror", (0.9, y + (0.06 if y > 0 else -0.06), 0.98), (0.12, 0.1, 0.08), "PAINT", 0.03))
        parts.append(box("handle", (0.2, y, 0.82), (0.16, 0.02, 0.03), CHROME))
        parts.append(box("handle2", (-0.85, y, 0.82), (0.16, 0.02, 0.03), CHROME))
    if kind == "taxi":
        parts.append(box("sign", (-0.25, 0, 1.52), (0.35, 0.6, 0.16), "EMIT_250_215_60", 0.04))
    if kind == "police":
        parts.append(box("bar_r", (-0.25, 0.32, 1.5), (0.25, 0.5, 0.12), "EMIT_255_40_40", 0.03))
        parts.append(box("bar_b", (-0.25, -0.32, 1.5), (0.25, 0.5, 0.12), "EMIT_40_80_255", 0.03))
    export(parts, os.path.join(OUT, f"{kind}.ocs"))


def minibus():
    clear()
    W = 2.0
    prof = [(-2.95, 0.36), (2.85, 0.36), (3.02, 0.55), (3.0, 1.0), (2.75, 1.32), (2.3, 2.2), (2.1, 2.28), (-2.85, 2.3), (-3.0, 2.15), (-3.02, 0.55)]
    body = body_mesh("body", prof, W, 9.0, 1.0, [0.45, 0.95, 1.15, 1.45, 2.05], [-2.6, -1.3, -1.2, 0.2, 0.3, 1.7, 1.8])
    rules = [
        (GLASS, lambda c, n: (1.45 < c.z < 2.05 and abs(n.y) > 0.6 and -2.6 < c.x < 2.4 and not (-1.3 < c.x < -1.2 or 0.2 < c.x < 0.3 or 1.7 < c.x < 1.8)) or (c.x > 2.2 and c.z > 1.3 and n.x > 0.3)),
        ("L8_30_130_70", lambda c, n: 0.95 < c.z < 1.15 and abs(n.y) > 0.6),
        (TRIM, lambda c, n: c.z < 0.45),
    ]
    finish(body, rules, (2.0, -1.95), 0.46, W)
    parts = [body]
    for y in (0.65, -0.65):
        parts.append(box("hl", (2.99, y, 0.82), (0.06, 0.32, 0.14), HEAD, 0.03))
        parts.append(box("tl", (-3.0, y, 0.9), (0.06, 0.22, 0.2), TAIL, 0.03))
    parts.append(box("rack", (-0.3, 0, 2.38), (4.2, 1.7, 0.06), TRIM))
    parts.append(box("bag1", (-0.8, 0.2, 2.6), (1.6, 1.0, 0.4), "L8_150_92_52", 0.05))
    parts.append(box("bag2", (0.8, -0.3, 2.55), (0.8, 0.7, 0.3), "L8_40_90_160", 0.05))
    parts.append(box("bag3", (1.2, 0.45, 2.52), (0.5, 0.5, 0.25), "L14_255_255_255", 0.04))
    export(parts, os.path.join(OUT, "minibus.ocs"))


def wheel():
    clear()
    parts = []
    t = cyl("tyre", (0, 0, 0), 0.34, 0.22, "L8_22_22_24", (math.pi / 2, 0, 0), 28)
    b = t.modifiers.new("b", "BEVEL")
    b.width = 0.05
    b.segments = 3
    parts.append(t)
    parts.append(cyl("rim", (0, 0.115, 0), 0.21, 0.02, CHROME, (math.pi / 2, 0, 0), 24))
    parts.append(cyl("hub", (0, 0.13, 0), 0.06, 0.03, "L8_120_122_128", (math.pi / 2, 0, 0), 12))
    for k in range(5):
        a = k / 5 * math.tau
        s = box("spoke", (0.1 * math.cos(a), 0.125, 0.1 * math.sin(a)), (0.2, 0.012, 0.04), "L8_150_152_158")
        s.rotation_euler = (0, -a, 0)
        parts.append(s)
    export(parts, os.path.join(OUT, "wheel.ocs"))


def variant(kind):
    """More cars of Ouaga, after the person's pictures: pickup, SUV, luxury saloon, old 504-style
    saloon, ambulance, driving-school car."""
    clear()
    W = 1.85
    lights = (2.2, -2.2, 0.64, 0.72)
    extra = []
    if kind == "pickup":
        W = 1.9
        prof = [(-2.6, 0.38), (2.45, 0.38), (2.58, 0.5), (2.6, 0.82), (2.45, 1.05), (1.4, 1.12), (1.0, 1.2), (0.62, 1.78), (-0.55, 1.8), (-0.66, 1.2), (-0.72, 1.06), (-2.55, 1.06), (-2.62, 0.82)]
        body = body_mesh("body", prof, W, 1.2, 0.9, [0.5, 1.2], [-0.6, 0.98])
        win = lambda c, n: c.z > 1.22 and n.z < 0.8 and -0.6 < c.x < 1.0
        arches, r = (1.6, -1.7), 0.44
        lights = (2.55, -2.6, 0.8, 0.85)
        # open load bed: dark floor and side rails
        extra.append(box("bed", (-1.62, 0, 1.075), (1.75, W - 0.25, 0.04), "L8_38_38_40"))
        for y in (W / 2 - 0.06, -W / 2 + 0.06):
            extra.append(box("rail", (-1.62, y, 1.12), (1.8, 0.08, 0.1), "PAINT", 0.02))
        extra.append(box("bags", (-1.4, 0.2, 1.35), (0.9, 0.8, 0.5), "L8_150_110_70", 0.08))
    elif kind == "suv":
        W = 1.95
        prof = [(-2.3, 0.42), (2.25, 0.42), (2.38, 0.58), (2.4, 0.88), (2.25, 1.06), (1.35, 1.14), (0.95, 1.22), (0.55, 1.8), (0.35, 1.87), (-2.05, 1.88), (-2.3, 1.76), (-2.38, 1.1), (-2.4, 0.62)]
        body = body_mesh("body", prof, W, 1.22, 0.9, [0.55, 1.22], [-2.15, -0.75, -0.62, 0.96])
        win = lambda c, n: c.z > 1.24 and n.z < 0.8 and -2.2 < c.x < 0.97 and not (-0.75 < c.x < -0.62 and abs(n.y) > 0.5)
        arches, r = (1.45, -1.45), 0.46
        lights = (2.36, -2.36, 0.88, 1.0)
        extra.append(box("spare", (-2.47, 0, 1.0), (0.16, 0.62, 0.62), "L8_25_25_28", 0.06))
        for y in (0.6, -0.6):
            extra.append(box("roofbar", (-0.6, y, 1.93), (2.2, 0.05, 0.05), "L8_40_40_42"))
    elif kind == "luxe":
        W = 1.9
        prof = [(-2.4, 0.28), (2.35, 0.28), (2.48, 0.4), (2.5, 0.58), (2.35, 0.7), (1.3, 0.8), (1.0, 0.86), (0.35, 1.3), (0.0, 1.36), (-1.0, 1.34), (-1.6, 1.08), (-2.3, 0.95), (-2.45, 0.78), (-2.48, 0.45)]
        body = body_mesh("body", prof, W, 0.86, 0.82, [0.42, 0.86], [-1.55, -0.3, -0.2, 0.98])
        win = lambda c, n: c.z > 0.88 and n.z < 0.85 and -1.58 < c.x < 1.0 and not (-0.3 < c.x < -0.2 and abs(n.y) > 0.5)
        arches, r = (1.45, -1.5), 0.42
        lights = (2.45, -2.42, 0.55, 0.78)
        extra.append(box("grille", (2.5, 0, 0.5), (0.04, 0.9, 0.16), "L8_190_192_198", 0.01))
    elif kind == "ancienne":
        W = 1.7
        prof = [(-2.25, 0.32), (2.15, 0.32), (2.25, 0.45), (2.26, 0.72), (2.15, 0.78), (1.05, 0.86), (0.85, 0.92), (0.45, 1.38), (-0.95, 1.4), (-1.35, 0.98), (-2.15, 0.95), (-2.26, 0.85), (-2.28, 0.45)]
        body = body_mesh("body", prof, W, 0.92, 0.86, [0.45, 0.92], [-1.3, -0.3, -0.2, 0.88])
        win = lambda c, n: c.z > 0.94 and n.z < 0.85 and -1.33 < c.x < 0.9 and not (-0.3 < c.x < -0.2 and abs(n.y) > 0.5)
        arches, r = (1.3, -1.3), 0.4
        lights = (2.24, -2.26, 0.62, 0.7)
        for x in (2.3, -2.32):
            extra.append(box("bumper", (x, 0, 0.42), (0.06, W + 0.04, 0.1), "L8_190_192_198", 0.02))
    elif kind == "ambulance":
        W = 2.0
        prof = [(-2.7, 0.36), (2.4, 0.36), (2.55, 0.55), (2.55, 0.95), (2.3, 1.25), (1.75, 2.05), (1.5, 2.2), (-2.6, 2.3), (-2.75, 2.15), (-2.78, 0.55)]
        body = body_mesh("body", prof, W, 9.0, 1.0, [0.45, 0.95, 1.15, 1.35, 1.95], [-2.0, 0.6, 1.3])
        win = lambda c, n: (c.x > 1.5 and c.z > 1.3 and n.x > 0.3) or (0.6 < c.x < 1.75 and 1.35 < c.z < 1.95 and abs(n.y) > 0.6)
        arches, r = (1.75, -1.8), 0.46
        lights = (2.53, -2.76, 0.8, 0.95)
        extra.append(box("bar_r", (1.3, 0.4, 2.33), (0.3, 0.6, 0.14), "EMIT_255_40_40", 0.03))
        extra.append(box("bar_b", (1.3, -0.4, 2.33), (0.3, 0.6, 0.14), "EMIT_60_110_255", 0.03))
        for y in (W / 2 + 0.005, -W / 2 - 0.005):
            extra.append(box("cross_v", (-0.8, y, 1.65), (0.16, 0.01, 0.6), "L8_200_30_30"))
            extra.append(box("cross_h", (-0.8, y, 1.65), (0.6, 0.01, 0.16), "L8_200_30_30"))
    else:  # autoecole: a small saloon with the driving-school sign on the roof
        prof = [(-2.0, 0.3), (1.95, 0.3), (2.06, 0.42), (2.08, 0.6), (2.0, 0.74), (1.7, 0.82), (1.0, 0.88), (0.8, 0.95), (0.35, 1.34), (0.15, 1.4), (-0.9, 1.4), (-1.15, 1.3), (-1.85, 0.95), (-2.05, 0.8), (-2.08, 0.5)]
        W = 1.72
        body = body_mesh("body", prof, W, 0.9, 0.84, [0.45, 0.9], [-1.5, -0.45, -0.35, 0.8])
        win = lambda c, n: c.z > 0.92 and n.z < 0.8 and -1.52 < c.x < 0.82 and not (-0.45 < c.x < -0.35 and abs(n.y) > 0.5)
        arches, r = (1.25, -1.25), 0.4
        lights = (2.04, -2.05, 0.64, 0.72)
        extra.append(box("sign", (-0.3, 0, 1.55), (0.3, 0.75, 0.24), "L8_245_245_245", 0.03))
        for y in (0.381, -0.381):
            extra.append(box("sign_band", (-0.3, y, 1.55), (0.25, 0.01, 0.1), "L8_200_30_30"))
    rules = [(GLASS, win), (TRIM, lambda c, n: c.z < 0.45)]
    if kind == "ambulance":
        rules.append(("L8_200_30_30", lambda c, n: 0.95 < c.z < 1.15 and abs(n.y) > 0.6))
        rules.append(("L8_242_242_240", lambda c, n: True))
    if kind == "autoecole":
        rules.append(("L8_240_240_238", lambda c, n: True))
    finish(body, rules, arches, r, W)
    parts = [body] + extra
    fx, bx, hz, tz = lights
    for y in (W / 2 - 0.35, -W / 2 + 0.35):
        parts.append(box("hl", (fx, y, hz), (0.07, 0.34, 0.1), HEAD, 0.03))
        parts.append(box("tl", (bx, y, tz), (0.07, 0.3, 0.12), TAIL, 0.03))
    for y in (W / 2 + 0.03, -W / 2 - 0.03):
        parts.append(box("mirror", (0.85 if kind != "pickup" else 0.95, y + (0.06 if y > 0 else -0.06), 1.0 if kind not in ("suv", "pickup", "ambulance") else 1.3), (0.12, 0.1, 0.09), TRIM, 0.03))
    export(parts, os.path.join(OUT, f"{kind}.ocs"))


for k in ("sedan", "taxi", "police"):
    car(k)
minibus()
wheel()
for k in ("pickup", "suv", "luxe", "ancienne", "ambulance", "autoecole"):
    variant(k)
