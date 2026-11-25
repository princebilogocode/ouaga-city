# Render a preview of OCS1 meshes with Blender (Workbench): blender -b -P preview.py -- out.png a.ocs [b.ocs dx]...
import bpy, sys, struct, math
from mathutils import Vector

args = sys.argv[sys.argv.index("--") + 1:]
OUT = args[0]
files = args[1:]
bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene
offset = 0.0
allp = []
for f in files:
    data = open(f, "rb").read()
    n = struct.unpack_from("<I", data, 4)[0]
    vs = []
    cols = []
    for i in range(n):
        v = struct.unpack_from("<13f", data, 8 + i * 52)
        vs.append((v[0] + offset, -v[2], v[1]))
        cols.append(v[6:10])
    faces = [(i, i + 1, i + 2) for i in range(0, n, 3)]
    me = bpy.data.meshes.new(f)
    me.from_pydata(vs, [], faces)
    attr = me.color_attributes.new("col", "FLOAT_COLOR", "CORNER")
    for li, loop in enumerate(me.loops):
        c = cols[loop.vertex_index]
        attr.data[li].color = (c[0], c[1], 0.0 if (c[0] == 1 and c[2] == 1 and c[1] == 0) else c[2], 1)
    o = bpy.data.objects.new(f, me)
    sc.collection.objects.link(o)
    allp += vs
    offset += 7.0
lo = Vector((min(p[0] for p in allp), min(p[1] for p in allp), min(p[2] for p in allp)))
hi = Vector((max(p[0] for p in allp), max(p[1] for p in allp), max(p[2] for p in allp)))
c = (lo + hi) / 2
size = (hi - lo).length
cam = bpy.data.cameras.new("c")
co = bpy.data.objects.new("c", cam)
sc.collection.objects.link(co)
co.location = c + Vector((size * 0.55, -size * 0.75, size * 0.45))
d = c - co.location
co.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
sc.camera = co
sc.render.engine = "BLENDER_WORKBENCH"
sc.display.shading.color_type = "VERTEX"
sc.display.shading.light = "STUDIO"
sc.display.shading.show_shadows = True
sc.render.resolution_x = 1280
sc.render.resolution_y = 720
sc.render.filepath = OUT
bpy.ops.render.render(write_still=True)
