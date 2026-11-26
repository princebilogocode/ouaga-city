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


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    image, name = os.path.abspath(sys.argv[1]), sys.argv[2]
    kind = sys.argv[3] if len(sys.argv) > 3 else "objet"
    out = os.path.join(HERE, "out", name)
    if os.path.exists(out):
        shutil.rmtree(out)
    env = dict(os.environ, HF_HOME=os.path.join(HERE, "cache"))
    print("1/3 TripoSR (processeur)…", flush=True)
    subprocess.run([VENV_PY, "run.py", image, "--device", "cpu", "--output-dir", out,
                    "--mc-resolution", "256" if kind == "personnage" else "192",
                    "--bake-texture", "--texture-resolution", "1024"], cwd=TRIPOSR, env=env, check=True)
    obj = os.path.join(out, "0", "mesh.obj")
    shutil.copy(os.path.join(out, "0", "mesh.glb"), obj)  # TripoSR writes OBJ text under this name
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
