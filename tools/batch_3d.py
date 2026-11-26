"""Convert every vehicle / moto / building picture of assets/user to a 3D model, on this computer (TripoSR + Blender).
    python tools/batch_3d.py            -> all, skipping those already done
"""
import os, subprocess, sys, glob
HERE = os.path.dirname(os.path.abspath(__file__)); APP = os.path.dirname(HERE)
pics = []
for pat in ("voiture_*.png", "moto_*.png", "bus_*.png", "camion_*.png", "minibus_*.png", "velo.png", "bat_*.png"):
    pics += sorted(glob.glob(os.path.join(APP, "assets", "user", pat)))
if len(sys.argv) > 1:
    pics = [p for p in pics if any(a in p for a in sys.argv[1:])]
for p in pics:
    name = "x_" + os.path.splitext(os.path.basename(p))[0]
    if os.path.exists(os.path.join(APP, "assets", "models", name + ".glb")):
        continue
    print("==>", name, flush=True)
    r = subprocess.run([sys.executable, os.path.join(HERE, "image_vers_3d.py"), p, name, "objet"])
    print(name, "ok" if r.returncode == 0 else "ERREUR", flush=True)
print("fini")
