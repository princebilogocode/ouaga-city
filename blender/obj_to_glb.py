# Blender script: TripoSR output (OBJ + texture.png) -> clean, upright, lighter GLB.
# Usage: blender -b -P obj_to_glb.py -- <mesh.obj> <texture.png> <out.glb> <personnage|objet> [faces]
import bpy, bmesh, sys, math
from mathutils import Vector, Matrix

a = sys.argv[sys.argv.index("--") + 1:]
OBJ, TEX, OUT, KIND = a[0], a[1], a[2], a[3]
FACES = int(a[4]) if len(a) > 4 else 6000

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.wm.obj_import(filepath=OBJ)
objs = [o for o in bpy.context.scene.objects if o.type == "MESH"]
for o in objs:
    o.select_set(True)
bpy.context.view_layer.objects.active = objs[0]
if len(objs) > 1:
    bpy.ops.object.join()
o = bpy.context.view_layer.objects.active
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
me = o.data

# clean: merge duplicates, drop loose bits, recompute normals
bm = bmesh.new()
bm.from_mesh(me)
bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-4)
# keep the biggest connected piece and those bigger than 2 % of it (removes floating crumbs)
bm.verts.ensure_lookup_table()
seen = set()
islands = []
for v in bm.verts:
    if v.index in seen:
        continue
    stack = [v]
    comp = []
    seen.add(v.index)
    while stack:
        x = stack.pop()
        comp.append(x)
        for e in x.link_edges:
            y = e.other_vert(x)
            if y.index not in seen:
                seen.add(y.index)
                stack.append(y)
    islands.append(comp)
islands.sort(key=len, reverse=True)
if islands:
    keep = len(islands[0]) * 0.02
    drop = [v for comp in islands if len(comp) < keep for v in comp]
    bmesh.ops.delete(bm, geom=drop, context="VERTS")
bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
bm.to_mesh(me)
bm.free()

# upright: a standing person is taller than wide; TripoSR often lays it down
co = [v.co for v in me.vertices]
ext = Vector((max(c.x for c in co) - min(c.x for c in co), max(c.y for c in co) - min(c.y for c in co), max(c.z for c in co) - min(c.z for c in co)))
if KIND == "personnage":
    longest = max(range(3), key=lambda i: ext[i])
    # TripoSR's convention: the figure lies along +Y with its head towards -Y
    if longest == 1:
        me.transform(Matrix.Rotation(math.radians(-90), 4, "X"))
    elif longest == 0:
        me.transform(Matrix.Rotation(math.radians(-90), 4, "Y"))
me.update()

# lighter mesh for the game
n = len(me.polygons)
if n > FACES:
    d = o.modifiers.new("dec", "DECIMATE")
    d.ratio = FACES / n
    bpy.ops.object.modifier_apply(modifier="dec")
for p in me.polygons:
    p.use_smooth = True

# texture material (picked up by the game and by rig_character.py)
m = bpy.data.materials.new("tex")
m.use_nodes = True
bsdf = m.node_tree.nodes["Principled BSDF"]
im = m.node_tree.nodes.new("ShaderNodeTexImage")
im.image = bpy.data.images.load(TEX)
m.node_tree.links.new(im.outputs["Color"], bsdf.inputs["Base Color"])
me.materials.clear()
me.materials.append(m)

bpy.ops.export_scene.gltf(filepath=OUT, export_format="GLB", use_selection=False)
print("wrote", OUT, len(me.polygons), "faces")
