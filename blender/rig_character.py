# Blender script: rig a generated character (static GLB from fal.ai/Tripo) and bake its animations.
# Usage: blender -b -P rig_character.py -- <in.glb> <out.ocm> <height_m> <seed>
# Output (little endian):
#   b"OCM1", u32 nverts, u32 nbones, u32 nanims
#   nverts * 16 f32: pos3 nrm3 uv2 joints4 weights4   (game space: Y up, facing +X, right +Z, metres)
#   nbones * (u32 len + name utf8)
#   nanims * (u32 len + name, u32 frames, f32 fps, frames*nbones*12 f32 (3x4 skin matrices, row major))
import bpy, bmesh, sys, struct, math
from mathutils import Vector, Matrix

argv = sys.argv[sys.argv.index("--") + 1:]
SRC, OUT, H = argv[0], argv[1], float(argv[2])

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=SRC)
meshes = [o for o in bpy.context.scene.objects if o.type == "MESH"]
for o in bpy.context.scene.objects:
    o.select_set(False)
for o in meshes:
    o.select_set(True)
bpy.context.view_layer.objects.active = meshes[0]
bpy.ops.object.parent_clear(type="CLEAR_KEEP_TRANSFORM")
if len(meshes) > 1:
    bpy.ops.object.join()
obj = bpy.context.view_layer.objects.active
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
me = obj.data

# --- clean the mesh with Blender: merge duplicate vertices, consistent normals
bm = bmesh.new()
bm.from_mesh(me)
bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
bm.to_mesh(me)
bm.free()

# --- normalise: feet at z = 0, height H, centred, facing +X
co = [v.co.copy() for v in me.vertices]
zmin = min(c.z for c in co); zmax = max(c.z for c in co)
xmin = min(c.x for c in co); xmax = max(c.x for c in co)
ymin = min(c.y for c in co); ymax = max(c.y for c in co)
s = H / (zmax - zmin)
cx, cy = (xmin + xmax) / 2, (ymin + ymax) / 2
for v in me.vertices:
    v.co = Vector(((v.co.x - cx) * s, (v.co.y - cy) * s, (v.co.z - zmin) * s))
feet = [v.co for v in me.vertices if v.co.z < 0.05 * H]
if sum(c.x for c in feet) / max(1, len(feet)) < 0:
    for v in me.vertices:
        v.co = Vector((-v.co.x, -v.co.y, v.co.z))
me.update()

# --- skeleton proportions measured on the mesh
def side_pts(sg, cond):
    return [v.co for v in me.vertices if sg * v.co.y > 0 and cond(v.co)]

J = {}
J["hips"] = Vector((0, 0, 0.52 * H))
J["spine"] = Vector((0, 0, 0.60 * H))
J["chest"] = Vector((0, 0, 0.71 * H))
J["neck"] = Vector((0, 0, 0.815 * H))
J["head"] = Vector((0, 0, 0.865 * H))
J["head_end"] = Vector((0, 0, 1.0 * H))
for sg, n in ((1, "L"), (-1, "R")):
    pts = side_pts(sg, lambda c: c.z > 0.35 * H)
    tip = max(pts, key=lambda c: sg * c.y)
    sh = Vector((0.0, sg * 0.095 * H, 0.81 * H))
    d = tip - sh
    J["shoulder" + n] = sh
    J["elbow" + n] = sh + d * 0.47 + Vector((-0.012 * H, 0, 0))
    J["wrist" + n] = sh + d * 0.83
    J["tip" + n] = tip
    foot = side_pts(sg, lambda c: c.z < 0.06 * H)
    fy = sum(c.y for c in foot) / max(1, len(foot))
    fx = max(c.x for c in foot)
    J["hipj" + n] = Vector((0, sg * 0.055 * H, 0.50 * H))
    J["knee" + n] = Vector((0.012 * H, (fy + sg * 0.055 * H) / 2, 0.275 * H))
    J["ankle" + n] = Vector((-0.01 * H, fy, 0.045 * H))
    J["toe" + n] = Vector((fx, fy, 0.012 * H))

BONES = [
    ("hips", "hips", "spine", None),
    ("spine", "spine", "chest", "hips"),
    ("chest", "chest", "neck", "spine"),
    ("neck", "neck", "head", "chest"),
    ("head", "head", "head_end", "neck"),
]
for n in ("L", "R"):
    BONES += [
        ("upperarm" + n, "shoulder" + n, "elbow" + n, "chest"),
        ("forearm" + n, "elbow" + n, "wrist" + n, "upperarm" + n),
        ("hand" + n, "wrist" + n, "tip" + n, "forearm" + n),
        ("thigh" + n, "hipj" + n, "knee" + n, "hips"),
        ("shin" + n, "knee" + n, "ankle" + n, "thigh" + n),
        ("foot" + n, "ankle" + n, "toe" + n, "shin" + n),
    ]
NAMES = [b[0] for b in BONES]
IDX = {n: i for i, n in enumerate(NAMES)}

# --- Blender armature + automatic (bone heat) weights, with a geometric fallback
arm_data = bpy.data.armatures.new("rig")
arm = bpy.data.objects.new("rig", arm_data)
bpy.context.scene.collection.objects.link(arm)
bpy.context.view_layer.objects.active = arm
bpy.ops.object.mode_set(mode="EDIT")
for name, h, t, parent in BONES:
    eb = arm_data.edit_bones.new(name)
    eb.head = J[h]
    eb.tail = J[t]
    if parent:
        eb.parent = arm_data.edit_bones[parent]
        eb.use_connect = False
bpy.ops.object.mode_set(mode="OBJECT")
for o in bpy.context.scene.objects:
    o.select_set(False)
obj.select_set(True)
arm.select_set(True)
bpy.context.view_layer.objects.active = arm
heat_ok = True
try:
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")
except Exception as e:
    heat_ok = False
    print("bone heat failed:", e)

def seg_dist(p, a, b):
    ab = b - a
    t = max(0.0, min(1.0, (p - a).dot(ab) / max(ab.length_squared, 1e-9)))
    return (a + ab * t - p).length

def geometric_weights(p):
    sg = 1 if p.y >= 0 else -1
    n = "L" if sg > 0 else "R"
    if sg * p.y > 0.115 * H and p.z > 0.36 * H:
        cand = ["upperarm" + n, "forearm" + n, "hand" + n, "chest"]
    elif p.z < 0.5 * H:
        cand = ["thighL", "shinL", "footL", "thighR", "shinR", "footR", "hips"]
    else:
        cand = ["hips", "spine", "chest", "neck", "head", "upperarmL", "upperarmR"]
    ds = sorted(((seg_dist(p, J[BONES[IDX[c]][1]], J[BONES[IDX[c]][2]]), c) for c in cand))[:3]
    ws = [(1.0 / (d + 0.01 * H) ** 4, c) for d, c in ds]
    tot = sum(w for w, _ in ws)
    return [(IDX[c], w / tot) for w, c in ws]

groups = {g.index: g.name for g in obj.vertex_groups}
vweights = []
bad = 0
for v in me.vertices:
    ws = [(IDX[groups[g.group]], g.weight) for g in v.groups if g.group in groups and groups[g.group] in IDX and g.weight > 1e-4]
    if not heat_ok or sum(w for _, w in ws) < 0.5:
        bad += 1
        ws = geometric_weights(v.co)
    ws = sorted(ws, key=lambda x: -x[1])[:4]
    tot = sum(w for _, w in ws)
    ws = [(i, w / tot) for i, w in ws]
    while len(ws) < 4:
        ws.append((0, 0.0))
    vweights.append(ws)
print("vertices without heat weights:", bad, "/", len(me.vertices))

# --- animations: rotations about armature axes at each joint, forward kinematics
def R(axis, deg):
    return Matrix.Rotation(math.radians(deg), 4, axis)

def arm_down(n):
    sg = 1 if n == "L" else -1
    d = J["elbow" + n] - J["shoulder" + n]
    a = math.degrees(math.atan2(sg * d.y, -d.z))
    return -sg * (a - 6.0)

def pose(rot, root=Vector((0, 0, 0))):
    D = {}
    for name, h, t, parent in BONES:
        hp = J[h]
        local = Matrix.Translation(hp) @ rot.get(name, Matrix.Identity(4)) @ Matrix.Translation(-hp)
        D[name] = (D[parent] if parent else Matrix.Translation(root)) @ local
    return [D[n] for n in NAMES]

def base_arms(rot, swingL=0.0, swingR=0.0, elbowL=8.0, elbowR=8.0):
    rot["upperarmL"] = R("Y", -swingL) @ R("X", arm_down("L"))
    rot["upperarmR"] = R("Y", -swingR) @ R("X", arm_down("R"))
    rot["forearmL"] = R("Y", -elbowL)
    rot["forearmR"] = R("Y", -elbowR)

def legs(rot, thL, thR, knL, knR, ftL=0.0, ftR=0.0):
    rot["thighL"] = R("Y", -thL); rot["thighR"] = R("Y", -thR)
    rot["shinL"] = R("Y", knL); rot["shinR"] = R("Y", knR)
    rot["footL"] = R("Y", ftL); rot["footR"] = R("Y", ftR)

def anim_idle(t):
    rot = {}
    b = math.sin(t * 2 * math.pi)
    base_arms(rot, 2 + b, 2 - b, 10, 10)
    rot["chest"] = R("Y", 1.2 * b)
    rot["head"] = R("Z", 4 * math.sin(t * math.pi))
    legs(rot, 0, 0, 2, 2)
    return pose(rot)

def anim_walk(t):
    rot = {}
    ph = t * 2 * math.pi
    sw = math.sin(ph)
    base_arms(rot, -22 * sw, 22 * sw, 14 + 8 * max(0, -sw), 14 + 8 * max(0, sw))
    legs(rot, 26 * sw, -26 * sw, 6 + 42 * max(0, math.sin(ph + 1.9)), 6 + 42 * max(0, math.sin(ph + 1.9 + math.pi)), -8 * sw, 8 * sw)
    rot["spine"] = R("Z", 5 * sw) @ R("Y", 3)
    rot["chest"] = R("Z", -7 * sw)
    rot["head"] = R("Z", 3 * sw)
    return pose(rot, Vector((0, 0, -0.012 * H * abs(math.cos(ph)))))

def anim_run(t):
    rot = {}
    ph = t * 2 * math.pi
    sw = math.sin(ph)
    base_arms(rot, -48 * sw, 48 * sw, 80, 80)
    legs(rot, 48 * sw, -48 * sw, 15 + 85 * max(0, math.sin(ph + 1.6)), 15 + 85 * max(0, math.sin(ph + 1.6 + math.pi)), -15 * sw, 15 * sw)
    rot["spine"] = R("Y", 12) @ R("Z", 8 * sw)
    rot["chest"] = R("Z", -10 * sw)
    return pose(rot, Vector((0, 0, -0.03 * H + 0.03 * H * abs(math.sin(ph)))))

def anim_aim(t):
    rot = {}
    b = math.sin(t * 2 * math.pi)
    base_arms(rot, 55, 0, 70, 0)
    # right arm straight forward, left hand supporting
    rot["upperarmR"] = R("Z", 8) @ R("Y", -86) @ R("X", arm_down("R"))
    rot["forearmR"] = R("Y", -4)
    rot["upperarmL"] = R("Z", -38) @ R("Y", -70) @ R("X", arm_down("L"))
    rot["forearmL"] = R("Y", -40) @ R("Z", -30)
    rot["chest"] = R("Z", -6 + 0.5 * b)
    legs(rot, 8, -6, 6, 10)
    return pose(rot)

def anim_ride(t):
    rot = {}
    base_arms(rot, 62, 62, 28, 28)
    legs(rot, 78, 78, 95, 95, -15, -15)
    rot["spine"] = R("Y", 18)
    rot["head"] = R("Y", -14)
    return pose(rot)

def anim_drive(t):
    rot = {}
    base_arms(rot, 55, 55, 45, 45)
    legs(rot, 88, 88, 80, 80)
    rot["spine"] = R("Y", -6)
    return pose(rot)

def anim_sit(t):
    rot = {}
    b = math.sin(t * 2 * math.pi)
    base_arms(rot, 22, 22, 55, 55)
    legs(rot, 90, 90, 90, 90)
    rot["chest"] = R("Y", 1.5 * b)
    return pose(rot)

def anim_eat(t):
    rot = {}
    k = max(0.0, math.sin(t * 2 * math.pi)) ** 0.7
    base_arms(rot, 22, 25 + 45 * k, 55, 60 + 85 * k)
    rot["upperarmR"] = R("Z", 25 * k) @ rot["upperarmR"]
    legs(rot, 90, 90, 90, 90)
    rot["head"] = R("Y", 8 * k)
    return pose(rot)

def anim_squat(t):
    rot = {}
    k = 0.5 - 0.5 * math.cos(t * 2 * math.pi)
    base_arms(rot, 85 * k, 85 * k, 5, 5)
    legs(rot, 95 * k, 95 * k, 115 * k, 115 * k, -25 * k, -25 * k)
    rot["spine"] = R("Y", 25 * k)
    return pose(rot, Vector((0.0, 0, -0.27 * H * k)))

def anim_punch(t):
    rot = {}
    k = math.sin(min(1.0, t * 2.0) * math.pi)
    base_arms(rot, 50, 20 + 70 * k, 110, 100 - 95 * k)
    rot["chest"] = R("Z", 25 * k)
    legs(rot, 15, -12, 12, 12)
    return pose(rot)

def anim_talk(t):
    rot = {}
    a = math.sin(t * 2 * math.pi)
    c = math.sin(t * 4 * math.pi + 1)
    base_arms(rot, 18 + 14 * a, 24 + 18 * c, 45 + 25 * a, 60 + 30 * c)
    rot["head"] = R("Z", 8 * math.sin(t * 2 * math.pi + 0.5)) @ R("Y", 4 * c)
    legs(rot, 0, 0, 3, 3)
    return pose(rot)

ANIMS = [
    ("idle", anim_idle, 60, True),
    ("walk", anim_walk, 32, True),
    ("run", anim_run, 20, True),
    ("aim", anim_aim, 30, True),
    ("ride", anim_ride, 2, True),
    ("drive", anim_drive, 2, True),
    ("sit", anim_sit, 60, True),
    ("eat", anim_eat, 60, True),
    ("squat", anim_squat, 48, True),
    ("punch", anim_punch, 15, False),
    ("talk", anim_talk, 60, True),
]

# Blender (x, y, z) -> game (x, z, -y)
C = Matrix(((1, 0, 0, 0), (0, 0, 1, 0), (0, -1, 0, 0), (0, 0, 0, 1)))
Ci = C.inverted()

# --- export: triangulated, per corner (UVs live on the corners)
bm = bmesh.new()
bm.from_mesh(me)
bmesh.ops.triangulate(bm, faces=bm.faces)
uvl = bm.loops.layers.uv.active
out = bytearray(b"OCM1")
tris = list(bm.faces)
out += struct.pack("<III", len(tris) * 3, len(NAMES), len(ANIMS))
for f in tris:
    for loop in f.loops:
        v = loop.vert
        p = v.co
        n = loop.calc_normal() if f.smooth is False else v.normal
        uv = loop[uvl].uv if uvl else Vector((0, 0))
        ws = vweights[v.index]
        out += struct.pack("<16f", p.x, p.z, -p.y, n.x, n.z, -n.y, uv.x, 1.0 - uv.y,
                           *[float(i) for i, _ in ws], *[w for _, w in ws])
bm.free()
for n in NAMES:
    b = n.encode()
    out += struct.pack("<I", len(b)) + b
for name, fn, frames, loop in ANIMS:
    b = name.encode()
    out += struct.pack("<I", len(b)) + b + struct.pack("<If", frames, 30.0)
    for k in range(frames):
        t = k / frames if loop else k / max(1, frames - 1)
        for m in fn(t):
            g = C @ m @ Ci
            for r in range(3):
                out += struct.pack("<4f", g[r][0], g[r][1], g[r][2], g[r][3])
open(OUT, "wb").write(out)
print("wrote", OUT, len(tris), "triangles, heat weights:", heat_ok)
