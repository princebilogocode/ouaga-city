import json, math, sys
LAT0, LON0 = 12.365, -1.52
KX = 111320*math.cos(math.radians(LAT0)); KY = 110540
def p(lat, lon): return (round((lon-LON0)*KX), round(-(lat-LAT0)*KY))
def simp(pts, tol=3.0):
    if len(pts) > 3 and pts[0] == pts[-1]:
        h = len(pts)//2
        return simp(pts[:h+1], tol)[:-1] + simp(pts[h:], tol)
    return simp0(pts, tol)
def simp0(pts, tol=3.0):
    # Douglas-Peucker
    if len(pts) < 3: return pts
    a, b = pts[0], pts[-1]
    dx, dy = b[0]-a[0], b[1]-a[1]; L = math.hypot(dx, dy) or 1
    best, bi = 0, 0
    for i in range(1, len(pts)-1):
        d = abs(dy*(pts[i][0]-a[0]) - dx*(pts[i][1]-a[1]))/L
        if d > best: best, bi = d, i
    if best > tol: return simp0(pts[:bi+1], tol)[:-1] + simp0(pts[bi:], tol)
    return [a, b]
kinds = {'motorway':0,'trunk':0,'trunk_link':0,'primary':1,'primary_link':1,'secondary':2,'tertiary':3,'unclassified':4,'residential':4}
out = []
els = json.load(open('ouaga.json', encoding='utf-8'))['elements']
for e in els:
    if e['type'] != 'way' or 'geometry' not in e: continue
    t = e.get('tags', {})
    pts = simp([p(g['lat'], g['lon']) for g in e['geometry']])
    name = t.get('name', '').replace('|', ' ')
    coords = ' '.join(f'{x},{y}' for x, y in pts)
    if 'highway' in t:
        out.append(f"R{kinds[t['highway']]}|{name}|{coords}")
    elif t.get('natural') == 'water': out.append(f"W|{name}|{coords}")
    elif t.get('aeroway') == 'aerodrome': out.append(f"A|{name}|{coords}")
    else: out.append(f"G|{name}|{coords}")
rels = json.load(open('rel.json', encoding='utf-8'))['elements']
groups = []; seen = 0; kinds_r = ['W','W','W','G']; names_r = ['Barrage Ouaga 1','Barrage Ouaga 2','','']
cur = None
for e in rels:
    if e['type'] == 'relation':
        if cur is not None: groups.append(cur); cur = None
        continue
    if cur is None: cur = []
    cur.append([(g['lat'], g['lon']) for g in e['geometry']])
if cur: groups.append(cur)
for gi, ways in enumerate(groups):
    ways = [list(w) for w in ways]
    rings = []
    while ways:
        ring = ways.pop(0)
        changed = True
        while changed and ring[0] != ring[-1]:
            changed = False
            for i, w in enumerate(ways):
                if w[0] == ring[-1]: ring += w[1:]
                elif w[-1] == ring[-1]: ring += w[::-1][1:]
                elif w[-1] == ring[0]: ring = w + ring[1:]
                elif w[0] == ring[0]: ring = w[::-1] + ring[1:]
                else: continue
                ways.pop(i); changed = True; break
        rings.append(ring)
    for ring in rings:
        pts = simp([p(a, b) for a, b in ring], 4.0)
        out.append(f"{kinds_r[gi]}|{names_r[gi]}|" + ' '.join(f'{x},{y}' for x, y in pts))
for e in json.load(open('places.json', encoding='utf-8'))['elements']:
    t = e['tags']; c = e.get('center') or e
    k = t.get('place') and 'Q' or {'fuel':'F','hospital':'H','marketplace':'M','police':'P'}.get(t.get('amenity'), 'S')
    if 'railway' in t: k = 'T'
    x, y = p(c['lat'], c['lon'])
    out.append(f"{k}|{t.get('name','').replace('|',' ')}|{x},{y}")
open(sys.argv[1], 'w', encoding='utf-8').write('\n'.join(out))
from collections import Counter; print(Counter(l[:2] for l in out))
