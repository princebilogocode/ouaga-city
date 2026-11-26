# Shared exporter: Blender objects -> OCS1 static mesh for the game.
# Vertex: pos3 nrm3 col4 uv2 layer (13 f32), game space (Y up): blender (x, y, z) -> game (x, z, -y).
# Material name "L<layer>_<r>_<g>_<b>" sets the texture layer and colour; "PAINT" -> magenta
# (replaced by the vehicle colour in game); "EMIT_<r>_<g>_<b>" -> emissive.
import bpy, bmesh, struct
from mathutils import Vector


def mat(name):
    m = bpy.data.materials.get(name)
    if m is None:
        m = bpy.data.materials.new(name)
    return m


def decode(name):
    if name.startswith("PAINT"):
        return 8.0, (1.0, 0.0, 1.0, 1.0)
    p = name.split("_")
    if p[0] == "EMIT":
        return 40.0, (int(p[1]) / 255, int(p[2]) / 255, int(p[3]) / 255, 1.0)
    if p[0].startswith("L"):
        return float(p[0][1:]), (int(p[1]) / 255, int(p[2]) / 255, int(p[3]) / 255, 1.0)
    return 8.0, (0.8, 0.8, 0.8, 1.0)


def export(objects, path, uv_scale=2.0, apply_mods=True):
    deps = bpy.context.evaluated_depsgraph_get()
    out = bytearray(b"OCS1")
    verts = bytearray()
    count = 0
    for o in objects:
        if o.type != "MESH":
            continue
        ev = o.evaluated_get(deps) if apply_mods else o
        me = ev.to_mesh()
        bm = bmesh.new()
        bm.from_mesh(me)
        bmesh.ops.triangulate(bm, faces=bm.faces)
        mw = o.matrix_world
        nw = mw.to_3x3().inverted().transposed()
        mats = [s.material.name if s.material else "" for s in o.material_slots]
        for f in bm.faces:
            name = mats[f.material_index] if f.material_index < len(mats) else ""
            layer, col = decode(name)
            fn = (nw @ f.normal).normalized()
            ax = max(range(3), key=lambda i: abs(fn[i]))
            for loop in f.loops:
                p = mw @ loop.vert.co
                n = (nw @ (loop.vert.normal if f.smooth else f.normal)).normalized()
                if ax == 0:
                    uv = (p.y / uv_scale, p.z / uv_scale)
                elif ax == 1:
                    uv = (p.x / uv_scale, p.z / uv_scale)
                else:
                    uv = (p.x / uv_scale, p.y / uv_scale)
                verts += struct.pack("<13f", p.x, p.z, -p.y, n.x, n.z, -n.y, *col, uv[0], 1.0 - uv[1], layer)
                count += 1
        bm.free()
        ev.to_mesh_clear()
    out += struct.pack("<I", count) + verts
    open(path, "wb").write(out)
    print("wrote", path, count // 3, "triangles")
