"""Image -> 3D model for Ouaga City, entirely on this computer (no payment, no internet after setup).

    python tools/image_vers_3d.py <image.png> <nom> [personnage|objet]

1. TripoSR (open source, MIT licence) builds a textured mesh from the image, on the processor
   (the Quadro M1000M has only 2 GB of video memory: too little for the GPU).
2. Blender cleans it, stands it up, lightens it and exports assets/models/<nom>.glb.
3. For a person, Blender then rigs and animates it (assets/chars/<nom>.ocm), as for Bila.

About 2-3 minutes for an object and 4-5 minutes for a person, the first run downloads ~2.7 GB.
"""
import json
import os
import shutil
import struct
import subprocess
import sys
import io

HERE = os.path.dirname(os.path.abspath(__file__))
APP = os.path.dirname(HERE)
VENV_PY = os.path.join(HERE, "venv", "Scripts", "python.exe")
TRIPOSR = os.path.join(HERE, "TripoSR")
BLENDER = r"C:\Program Files\Blender Foundation\Blender 5.3\blender.exe"


def prepare(image, out_png):
    """Cut the object out of its plain studio background ourselves (flood fill from the borders),
    centre it at 85 % of the frame on TripoSR's neutral grey. rembg keeps light-grey backdrops."""
    from PIL import Image, ImageFilter
    import numpy as np
    from collections import deque
    im = Image.open(image).convert("RGB")
    im.thumbnail((768, 768))
    a = np.asarray(im).astype(np.int32)
    h, w, _ = a.shape
    edge = np.concatenate([a[0], a[-1], a[:, 0], a[:, -1]])
    bg = np.median(edge, axis=0)
    diff = np.abs(a - bg).sum(axis=2)
    seen = np.zeros((h, w), bool)
    q = deque([(0, x) for x in range(w)] + [(h - 1, x) for x in range(w)] + [(y, 0) for y in range(h)] + [(y, w - 1) for y in range(h)])
    tol = 34
    while q:
        y, x = q.popleft()
        if seen[y, x] or diff[y, x] > tol:
            continue
        seen[y, x] = True
        for yy, xx in ((y + 1, x), (y - 1, x), (y, x + 1), (y, x - 1)):
            if 0 <= yy < h and 0 <= xx < w and not seen[yy, xx]:
                q.append((yy, xx))
    mask = Image.fromarray(((~seen) * 255).astype(np.uint8)).filter(ImageFilter.MinFilter(3)).filter(ImageFilter.GaussianBlur(1))
    ys, xs = np.nonzero(np.asarray(mask) > 128)
    box = (xs.min(), ys.min(), xs.max() + 1, ys.max() + 1)
    obj = im.crop(box); m = mask.crop(box)
    side = int(max(obj.size) / 0.85)
    canvas = Image.new("RGB", (side, side), (127, 127, 127))
    canvas.paste(obj, ((side - obj.size[0]) // 2, (side - obj.size[1]) // 2), m)
    canvas.save(out_png)


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    image, name = os.path.abspath(sys.argv[1]), sys.argv[2]
    kind = sys.argv[3] if len(sys.argv) > 3 else "objet"
    out = os.path.join(HERE, "out", name)
    reuse = os.path.exists(os.path.join(out, "0", "mesh.obj")) and os.path.exists(os.path.join(out, "0", "texture.png"))
    if os.path.exists(out) and not reuse:
        shutil.rmtree(out)
    env = dict(os.environ, HF_HOME=os.path.join(HERE, "cache"))
    print("1/3 TripoSR (processeur)…", flush=True)
    if not reuse:
      os.makedirs(os.path.join(out, "0"), exist_ok=True)
      prepped = os.path.join(out, "prepared.png")
      prepare(image, prepped)
      subprocess.run([VENV_PY, "run.py", prepped, "--no-remove-bg", "--device", "cpu", "--output-dir", out,
                    "--mc-resolution", "256" if kind == "personnage" else "192",
                    "--bake-texture", "--texture-resolution", "1024"], cwd=TRIPOSR, env=env, check=True)
    obj = os.path.join(out, "0", "mesh.obj")
    if not os.path.exists(obj):
        shutil.copy(os.path.join(out, "0", "mesh.glb"), obj)  # older runs wrote OBJ text under this name
    glb = os.path.join(APP, "assets", "models", name + ".glb")
    print("2/3 Blender : nettoyage, redressement, allègement…", flush=True)
    subprocess.run([BLENDER, "-b", "-P", os.path.join(APP, "blender", "obj_to_glb.py"), "--", obj,
                    os.path.join(out, "0", "texture.png"), glb, kind, "5000" if kind == "personnage" else "4000"], check=True)
    if kind == "personnage":
        print("3/3 Blender : squelette, doigts et animations…", flush=True)
        ocm = os.path.join(APP, "assets", "chars", name + ".ocm")
        subprocess.run([BLENDER, "-b", "-P", os.path.join(APP, "blender", "rig_character.py"), "--", glb, ocm, "1.75"], check=True)
        # texture for the game
        from PIL import Image
        Image.open(os.path.join(out, "0", "texture.png")).convert("RGB").resize((1024, 1024)).save(
            os.path.join(APP, "assets", "chars", name + "_c.jpg"), quality=90)
        Image.new("RGB", (8, 8), (128, 128, 255)).save(os.path.join(APP, "assets", "chars", name + "_n.jpg"))
        print("Personnage prêt :", ocm)
    else:
        print("Objet prêt :", glb)


if __name__ == "__main__":
    main()
