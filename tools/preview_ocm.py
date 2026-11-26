# Render a rigged OCM character in a pose: blender -b -P preview_ocm.py -- file.ocm texture.jpg out.png anim frame [anim frame]...
import bpy, sys, struct, math
from mathutils import Vector, Matrix

a = sys.argv[sys.argv.index("--") + 1:]
OCM, TEX, OUT = a[0], a[1], a[2]
poses = [(a[i], int(a[i + 1])) for i in range(3, len(a), 2)]
data = open(OCM, "rb").read()
nv, nb, na = struct.unpack_from("<III", data, 4)
o = 16
verts = [struct.unpack_from("<16f", data, o + i * 64) for i in range(nv)]
o += nv * 64
names = []
for _ in range(nb):
    l = struct.unpack_from("<I", data, o)[0]; o += 4
    names.append(data[o:o + l].decode()); o += l
anims = {}
for _ in range(na):
    l = struct.unpack_from("<I", data, o)[0]; o += 4
    name = data[o:o + l].decode(); o += l
    frames, fps = struct.unpack_from("<If", data, o); o += 8
    mats = []
    for k in range(frames * nb):
        r = struct.unpack_from("<12f", data, o); o += 48
        mats.append(Matrix((r[0:4], r[4:8], r[8:12], (0, 0, 0, 1))))
    anims[name] = (frames, mats)

bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene
img = bpy.data.images.load(TEX)
mat = bpy.data.materials.new("m")
mat.use_nodes = True
nt = mat.node_tree
bsdf = next(n for n in nt.nodes if n.type == "BSDF_PRINCIPLED")
t = nt.nodes.new("ShaderNodeTexImage"); t.image = img
nt.links.new(t.outputs["Color"], bsdf.inputs["Base Color"])
for k, (an, fr) in enumerate(poses):
    frames, mats = anims[an]
    fr = fr % frames
    pos = []
    for v in verts:
        p = Vector((v[0], v[1], v[2], 1.0))
        acc = Vector((0, 0, 0, 0))
        for j in range(4):
            w = v[12 + j]
            if w > 0:
                acc += (mats[fr * nb + int(v[8 + j])] @ p) * w
        pos.append((acc.x + k * 1.2, -acc.z, acc.y))
    faces = [(i, i + 1, i + 2) for i in range(0, nv, 3)]
    me = bpy.data.meshes.new(an)
    me.from_pydata(pos, [], faces)
    uvl = me.uv_layers.new()
    for li, loop in enumerate(me.loops):
        v = verts[loop.vertex_index]
        uvl.data[li].uv = (v[6], 1 - v[7])
    me.materials.append(mat)
    ob = bpy.data.objects.new(an, me)
    sc.collection.objects.link(ob)
cam = bpy.data.objects.new("c", bpy.data.cameras.new("c")); sc.collection.objects.link(cam)
cx = (len(poses) - 1) * 0.6
cam.location = Vector((cx + 0.5, -4.5 - len(poses) * 1.1, 1.3))
cam.rotation_euler = (Vector((cx, 0, 0.95)) - cam.location).to_track_quat("-Z", "Y").to_euler()
sc.camera = cam
sun = bpy.data.objects.new("s", bpy.data.lights.new("s", "SUN")); sc.collection.objects.link(sun); sun.rotation_euler = (0.7, 0.2, 0.6)
w = bpy.data.worlds.new("w"); sc.world = w; w.color = (0.7, 0.7, 0.72)
sc.render.resolution_x = 400 * len(poses); sc.render.resolution_y = 600
sc.render.filepath = OUT
bpy.ops.render.render(write_still=True)
