"""Download Ouaga's restaurants, maquis, pharmacies, banks, shops and health centres from OpenStreetMap
(Overpass) and write assets/services.map (same coordinates as ouaga.map).
Tags: E eat/drink, K pharmacy, N bank, O shop, H health."""
import json, math, os, urllib.request, urllib.parse
APP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LAT0, LON0 = 12.365, -1.52
KX = 111320 * math.cos(math.radians(LAT0)); KY = 110540
def p(lat, lon): return (round((lon - LON0) * KX), round(-(lat - LAT0) * KY))
BB = "12.25,-1.66,12.47,-1.38"
q = f"""[out:json][timeout:120];
(
 nwr["amenity"~"^(restaurant|fast_food|cafe|bar|pub|maquis|ice_cream)$"]({BB});
 nwr["amenity"~"^(pharmacy|bank|hospital|clinic|doctors)$"]({BB});
 nwr["healthcare"~"^(hospital|clinic|centre)$"]({BB});
 nwr["shop"~"^(supermarket|convenience|clothes|mall|general|bakery|hairdresser|mobile_phone|hardware|tailor|beauty)$"]({BB});
);
out center tags;"""
for url in ("https://maps.mail.ru/osm/tools/overpass/api/interpreter", "https://overpass-api.de/api/interpreter"):
    try:
        data = urllib.request.urlopen(urllib.request.Request(url, data=urllib.parse.urlencode({"data": q}).encode(), headers={"User-Agent": "ouaga-city-game"}), timeout=180).read()
        break
    except Exception as e:
        print(url, e)
els = json.loads(data.decode("utf-8"))["elements"]
out = []
for e in els:
    t = e.get("tags", {})
    lat = e.get("lat") or e.get("center", {}).get("lat"); lon = e.get("lon") or e.get("center", {}).get("lon")
    if lat is None:
        continue
    a = t.get("amenity", ""); s = t.get("shop", ""); h = t.get("healthcare", "")
    if a in ("pharmacy",): k = "K"
    elif a == "bank": k = "N"
    elif a in ("hospital", "clinic", "doctors") or h: k = "H"
    elif a: k = "E"
    else: k = "O"
    sub = a or s or h
    name = t.get("name", "").replace("|", " ")
    x, y = p(lat, lon)
    out.append(f"{k}|{name}|{x},{y}|{sub}")
open(os.path.join(APP, "assets", "services.map"), "w", encoding="utf-8").write("\n".join(out) + "\n")
from collections import Counter
print(len(out), Counter(l[0] for l in out))
