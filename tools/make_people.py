"""Generate Ouaga residents with Blender + MPFB, rig and animate them (blender/make_human.py + rig_character.py)."""
import json, os, subprocess, sys
from PIL import Image
APP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
B = r"C:\Program Files\Blender Foundation\Blender 5.3\blender.exe"
W = os.path.join(APP, "tools", "humans")
PEOPLE = {
    "h_jeune": dict(gender=1.0, age=0.4, muscle=0.6, weight=0.45, height=0.6, skin="young_african_male", hair="short02", clothes=["male_casualsuit01", "shoes01"]),
    "h_ouvrier": dict(gender=1.0, age=0.55, muscle=0.7, weight=0.5, height=0.55, skin="middleage_african_male", hair="short01", clothes=["male_worksuit01", "shoes03"]),
    "h_cadre": dict(gender=1.0, age=0.6, muscle=0.5, weight=0.6, height=0.6, skin="middleage_african_male", hair="short03", clothes=["male_elegantsuit01", "shoes05"], tint={"elegantsuit": [0.35, 0.4, 0.6]}),
    "h_ancien": dict(gender=1.0, age=0.9, muscle=0.35, weight=0.55, height=0.45, skin="old_african_male", hair="short04", clothes=["male_casualsuit03", "shoes02"]),
    "h_sportif": dict(gender=1.0, age=0.35, muscle=0.85, weight=0.45, height=0.7, skin="young_african_male", hair="afro01", clothes=["male_casualsuit04", "shoes06"]),
    "f_etudiante": dict(gender=0.0, age=0.35, muscle=0.5, weight=0.45, height=0.5, skin="young_african_female", hair="braid01", clothes=["female_casualsuit01", "shoes04"]),
    "f_cadre": dict(gender=0.0, age=0.55, muscle=0.45, weight=0.5, height=0.55, skin="middleage_african_female", hair="ponytail01", clothes=["female_elegantsuit01", "shoes05"], tint={"ponytail": [0.09, 0.07, 0.06]}),
    "f_sportive": dict(gender=0.0, age=0.4, muscle=0.7, weight=0.4, height=0.6, skin="young_african_female", hair="ponytail01", clothes=["female_sportsuit01", "shoes06"], tint={"ponytail": [0.09, 0.07, 0.06]}),
    "f_maman": dict(gender=0.0, age=0.6, muscle=0.4, weight=0.7, height=0.45, skin="middleage_african_female", hair="bob02", clothes=["female_casualsuit02", "shoes02"], tint={"casualsuit02": [1.0, 0.75, 0.35], "bob02": [0.1, 0.08, 0.07]}),
    "f_grandmere": dict(gender=0.0, age=0.92, muscle=0.3, weight=0.6, height=0.4, skin="old_african_female", hair="short04", clothes=["female_casualsuit02", "shoes02"], tint={"casualsuit02": [0.55, 0.75, 1.0]}),
    # Bila, the hero, and his outfits (wardrobe at the apartment)
    "bila": dict(gender=1.0, age=0.42, muscle=0.62, weight=0.42, height=0.75, skin="young_african_male", hair="short02", clothes=["male_casualsuit06", "shoes01"], tint={"short02": [0.08, 0.06, 0.05]}, retex={"casualsuit06": os.path.join(W, "casualsuit06_clean.png")}, faces=18000, lift_uv={"casualsuit06": [0.58, 0.008]}),
    "bila_costume": dict(gender=1.0, age=0.42, muscle=0.62, weight=0.42, height=0.75, skin="young_african_male", hair="short02", clothes=["male_elegantsuit01", "shoes05"], tint={"elegantsuit": [0.25, 0.25, 0.28], "short02": [0.08, 0.06, 0.05]}, faces=14000),
    "bila_veste": dict(gender=1.0, age=0.42, muscle=0.62, weight=0.42, height=0.75, skin="young_african_male", hair="short02", clothes=["male_casualsuit05", "shoes06"], tint={"short02": [0.08, 0.06, 0.05]}, faces=14000),
}
only = sys.argv[1:] or [k for k in PEOPLE if not k.startswith("bila")]
for name in only:
    spec = dict(PEOPLE[name]); spec["faces"] = spec.get("faces", 7000)
    sp = os.path.join(W, name + ".json"); json.dump(spec, open(sp, "w"))
    glb = os.path.join(W, name + ".glb"); jn = os.path.join(W, name + "_joints.json")
    r = subprocess.run([B, "-b", "-P", os.path.join(APP, "blender", "make_human.py"), "--", sp, glb, jn], capture_output=True, text=True)
    print(name, [l for l in r.stdout.splitlines() if l.startswith("human")] or r.stdout[-800:], flush=True)
    h = 1.82 if spec["gender"] > 0.5 else 1.68
    h -= 0.06 if spec["age"] > 0.85 else 0.0
    r = subprocess.run([B, "-b", "-P", os.path.join(APP, "blender", "rig_character.py"), "--", glb, os.path.join(APP, "assets", "chars", name + ".ocm"), str(h), jn], capture_output=True, text=True)
    print(name, [l for l in r.stdout.splitlines() if l.startswith("wrote")] or r.stdout[-800:], flush=True)
    Image.open(os.path.join(W, name + "_atlas.png")).convert("RGB").resize((1024, 1024)).save(os.path.join(APP, "assets", "chars", name + "_c.jpg"), quality=90)
    Image.new("RGB", (8, 8), (128, 128, 255)).save(os.path.join(APP, "assets", "chars", name + "_n.jpg"))
print("done")
