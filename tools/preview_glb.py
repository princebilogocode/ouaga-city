import bpy, sys, math
from mathutils import Vector
a = sys.argv[sys.argv.index("--") + 1:]
bpy.ops.wm.read_factory_settings(use_empty=True)
import os
if a[0].endswith(".obj"):
    bpy.ops.wm.obj_import(filepath=a[0])
    tex = os.path.join(os.path.dirname(os.path.abspath(a[0])), "texture.png")
    m = bpy.data.materials.new("t"); m.use_nodes = True
    bsdf = m.node_tree.nodes["Principled BSDF"]
    im = m.node_tree.nodes.new("ShaderNodeTexImage"); im.image = bpy.data.images.load(tex)
    m.node_tree.links.new(im.outputs["Color"], bsdf.inputs["Base Color"])
    for o in bpy.context.scene.objects:
        if o.type == "MESH":
            o.data.materials.clear(); o.data.materials.append(m)
else:
    bpy.ops.import_scene.gltf(filepath=a[0])
objs = [o for o in bpy.context.scene.objects if o.type == "MESH"]
pts = [o.matrix_world @ Vector(c) for o in objs for c in o.bound_box]
lo = Vector((min(p.x for p in pts), min(p.y for p in pts), min(p.z for p in pts)))
hi = Vector((max(p.x for p in pts), max(p.y for p in pts), max(p.z for p in pts)))
c = (lo + hi) / 2; s = (hi - lo).length
sc = bpy.context.scene
for i, (dx, dy) in enumerate(((0.9, -1.2), (-1.2, -0.6))):
    cam = bpy.data.objects.new("c", bpy.data.cameras.new("c")); sc.collection.objects.link(cam)
    cam.location = c + Vector((dx * s * 0.9, dy * s * 0.9, s * 0.5))
    cam.rotation_euler = (c - cam.location).to_track_quat("-Z", "Y").to_euler()
    sc.camera = cam
    sc.render.engine = "BLENDER_EEVEE" if "BLENDER_EEVEE" in [e.identifier for e in bpy.types.RenderSettings.bl_rna.properties["engine"].enum_items] else "BLENDER_EEVEE_NEXT"
    w = bpy.data.worlds.new("w"); sc.world = w; w.color = (0.8, 0.8, 0.82)
    sun = bpy.data.objects.new("s", bpy.data.lights.new("s", "SUN")); sc.collection.objects.link(sun); sun.rotation_euler = (0.6, 0.3, 0.8)
    sc.render.resolution_x = 640; sc.render.resolution_y = 640
    sc.render.filepath = a[1] + f"_{i}.png"
    bpy.ops.render.render(write_still=True)
