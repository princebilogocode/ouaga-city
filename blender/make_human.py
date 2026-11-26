# Blender + MPFB (MakeHuman for Blender, extensions.blender.org): build a realistic citizen of Ouaga.
# Usage: blender -b -P make_human.py -- <spec.json> <out.glb> <out_joints.json>
# spec: {"gender": 0..1, "age": 0..1, "muscle":.., "weight":.., "height":.., "skin": "young_african_male",
#        "hair": "afro01" | "", "clothes": ["male_casualsuit01", "shoes01"], "tint": {"male_casualsuit01": [r,g,b]}}
# Output: one mesh, one baked colour atlas (all materials), joints of MPFB's game-engine rig (fingers too).
import bpy, sys, os, json, math
from mathutils import Matrix, Vector

args = sys.argv[sys.argv.index("--") + 1:]
SPEC, OUT, JOINTS = args[0], args[1], args[2]
spec = json.load(open(SPEC, encoding="utf-8"))

import importlib
mpfb = importlib.import_module("bl_ext.user_default.mpfb")
from bl_ext.user_default.mpfb.services.humanservice import HumanService
from bl_ext.user_default.mpfb.services.locationservice import LocationService
from bl_ext.user_default.mpfb.services.targetservice import TargetService

bpy.ops.wm.read_homefile(use_empty=True)
DATA = LocationService.get_user_data()

macro = TargetService.get_default_macro_info_dict()
for k in ("gender", "age", "muscle", "weight", "height", "proportions"):
    if k in spec:
        macro[k] = spec[k]
macro["race"] = {"african": 1.0, "asian": 0.0, "caucasian": 0.0}
human = HumanService.create_human(macro_detail_dict=macro, feet_on_ground=True, scale=0.1)

def asset(kind, name, ext="mhclo"):
    return os.path.join(DATA, kind, name, f"{name}.{ext}")

HumanService.set_character_skin(asset("skins", spec.get("skin", "young_african_male"), "mhmat"), human, skin_type="MAKESKIN")
HumanService.add_mhclo_asset(os.path.join(DATA, "eyes", "low-poly", "low-poly.mhclo"), human, asset_type="Eyes", subdiv_levels=0, material_type="MAKESKIN")
HumanService.add_mhclo_asset(asset("eyebrows", spec.get("eyebrows", "eyebrow001")), human, asset_type="Eyebrows", subdiv_levels=0, material_type="MAKESKIN")
if spec.get("hair"):
    HumanService.add_mhclo_asset(asset("hair", spec["hair"]), human, asset_type="Hair", subdiv_levels=0, material_type="MAKESKIN")
for c in spec.get("clothes", []):
    HumanService.add_mhclo_asset(asset("clothes", c), human, asset_type="Clothes", subdiv_levels=0, material_type="MAKESKIN")

rig = HumanService.add_builtin_rig(human, "game_engine", import_weights=False)
rig = rig or next((o for o in bpy.context.scene.objects if o.type == "ARMATURE"), None)

# joints of the rig (head/tail of each bone), in object space
joints = {}
if rig:
    for b in rig.data.bones:
        joints[b.name] = {"head": list(rig.matrix_world @ b.head_local), "tail": list(rig.matrix_world @ b.tail_local)}

# keep only meshes; apply the masks (hidden body under clothes) and drop the armature modifier
meshes = [o for o in bpy.context.scene.objects if o.type == "MESH"]
for o in meshes:
    bpy.context.view_layer.objects.active = o
    for x in bpy.context.scene.objects:
        x.select_set(x == o)
    # bake the shape keys (macros) into the mesh first: modifiers cannot be applied with them
    if o.data.shape_keys:
        o.shape_key_add(name="final", from_mix=True)
        for k in list(o.data.shape_keys.key_blocks):
            if k.name != "final":
                o.shape_key_remove(k)
        o.shape_key_remove(o.data.shape_keys.key_blocks["final"])
    for m in list(o.modifiers):
        if m.type == "ARMATURE":
            o.modifiers.remove(m)
            continue
        try:
            bpy.ops.object.modifier_apply(modifier=m.name)
        except Exception as e:
            print("modifier", m.name, e)
            if m.name in o.modifiers:
                o.modifiers.remove(o.modifiers[m.name])
if rig:
    bpy.data.objects.remove(rig)

# tint clothes (e.g. dye a suit in a given colour) before baking
for o in meshes:
    for slot in o.material_slots:
        mat = slot.material
        if not mat or not mat.use_nodes:
            continue
        for key, rgb in spec.get("tint", {}).items():
            if key in mat.name or key in o.name:
                for n in mat.node_tree.nodes:
                    if n.type == "BSDF_PRINCIPLED":
                        mix = mat.node_tree.nodes.new("ShaderNodeMix")
                        mix.data_type = "RGBA"
                        mix.blend_type = "MULTIPLY"
                        mix.inputs["Factor"].default_value = 1.0
                        mix.inputs[7].default_value = (*rgb, 1.0)
                        src = n.inputs["Base Color"].links[0].from_socket if n.inputs["Base Color"].is_linked else None
                        if src:
                            mat.node_tree.links.new(src, mix.inputs[6])
                        else:
                            mix.inputs[6].default_value = n.inputs["Base Color"].default_value
                        mat.node_tree.links.new(mix.outputs[2], n.inputs["Base Color"])

# join everything into the body
for o in bpy.context.scene.objects:
    o.select_set(False)
for o in meshes:
    o.select_set(True)
bpy.context.view_layer.objects.active = human
bpy.ops.object.join()
body = bpy.context.view_layer.objects.active
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)

# one atlas for every material: new UV map, bake the diffuse colour into it (Cycles, CPU)
me = body.data
src_uv = me.uv_layers[0].name
atlas_uv = me.uv_layers.new(name="atlas")
me.uv_layers.active = atlas_uv
bpy.ops.object.mode_set(mode="EDIT")
bpy.ops.mesh.select_all(action="SELECT")
bpy.ops.uv.smart_project(angle_limit=math.radians(66), island_margin=0.003)
bpy.ops.object.mode_set(mode="OBJECT")
for uv in me.uv_layers:
    uv.active_render = uv.name == src_uv
img = bpy.data.images.new("atlas", 2048, 2048)
for slot in body.material_slots:
    mat = slot.material
    if not mat:
        continue
    mat.use_nodes = True
    nt = mat.node_tree
    for n in nt.nodes:
        n.select = False
    # sample the material's own textures with its own UV map
    for n in nt.nodes:
        if n.type == "TEX_IMAGE" and not n.inputs["Vector"].is_linked:
            uvn = nt.nodes.new("ShaderNodeUVMap")
            uvn.uv_map = src_uv
            nt.links.new(uvn.outputs["UV"], n.inputs["Vector"])
    t = nt.nodes.new("ShaderNodeTexImage")
    t.image = img
    uvt = nt.nodes.new("ShaderNodeUVMap")
    uvt.uv_map = "atlas"
    nt.links.new(uvt.outputs["UV"], t.inputs["Vector"])
    t.select = True
    nt.nodes.active = t
sc = bpy.context.scene
sc.render.engine = "CYCLES"
sc.cycles.device = "CPU"
sc.cycles.samples = 1
sc.render.bake.use_pass_direct = False
sc.render.bake.use_pass_indirect = False
sc.render.bake.margin = 4
bpy.ops.object.bake(type="DIFFUSE", pass_filter={"COLOR"}, use_clear=True)
tex_path = os.path.splitext(OUT)[0] + "_atlas.png"
img.filepath_raw = tex_path
img.file_format = "PNG"
img.save()

# a single clean material with the atlas, for export
for uv in list(me.uv_layers):
    if uv.name != "atlas":
        me.uv_layers.remove(uv)
mat = bpy.data.materials.new("atlas")
mat.use_nodes = True
nt = mat.node_tree
bsdf = next((n for n in nt.nodes if n.type == "BSDF_PRINCIPLED"), None)
if bsdf is None:
    bsdf = nt.nodes.new("ShaderNodeBsdfPrincipled")
    out = next((n for n in nt.nodes if n.type == "OUTPUT_MATERIAL"), None) or nt.nodes.new("ShaderNodeOutputMaterial")
    nt.links.new(bsdf.outputs[0], out.inputs[0])
t = mat.node_tree.nodes.new("ShaderNodeTexImage")
t.image = img
mat.node_tree.links.new(t.outputs["Color"], bsdf.inputs["Base Color"])
me.materials.clear()
me.materials.append(mat)
for p in me.polygons:
    p.material_index = 0

# lighter for the game: about 9 000 faces, keeping the hands' fingers
n = len(me.polygons)
target = spec.get("faces", 9000)
if n > target:
    d = body.modifiers.new("dec", "DECIMATE")
    d.ratio = target / n
    bpy.ops.object.modifier_apply(modifier="dec")

bpy.ops.export_scene.gltf(filepath=OUT, export_format="GLB", use_selection=False, export_skins=False, export_animations=False)
json.dump(joints, open(JOINTS, "w"))
print("human", OUT, len(me.polygons), "faces,", len(joints), "joints")
