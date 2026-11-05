#!/usr/bin/env python3
"""Erzeugt die mitgelieferte Objekt-Bibliothek (Cartoon-Stil) als
apps/studio/assets/bibliothek.json. Ursprung jedes Objekts: unten in der Mitte, Vorderseite +Z.
Aufruf: python3 tools/make_starter_library.py"""
import json, os

def rb(id, size, pos, color, r=0.08, **kw):
    r = round(min(r, min(size) / 2 * 0.9), 4)  # Rundung kleiner als die halbe kleinste Kante
    o = {"id": id, "type": "rounded_box", "size": size, "radius": r, "position": pos, "material": {"color": color, "roughness": kw.pop("rough", 0.7)}}
    o["material"].update(kw.pop("mat", {})); o.update(kw); return o

def cyl(id, r, h, pos, color, **kw):
    o = {"id": id, "type": "cylinder", "radius": r, "height": h, "position": pos, "material": {"color": color, "roughness": kw.pop("rough", 0.6)}}
    o["material"].update(kw.pop("mat", {})); o.update(kw); return o

def sph(id, r, pos, color, **kw):
    o = {"id": id, "type": "sphere", "radius": r, "position": pos, "material": {"color": color, "roughness": kw.pop("rough", 0.7)}}
    o["material"].update(kw.pop("mat", {})); o.update(kw); return o

def group(id, children, **kw):
    o = {"id": id, "type": "group", "children": children}; o.update(kw); return o

assets = []
def add(name, desc, obj):
    assets.append({"name": name, "description": desc, "object": obj})

# ---------- Haus ----------
win = lambda i, x: group(f"fenster_{i}", [
    rb(f"glas_{i}", [0.8, 0.8, 0.06], [0, 0, 0], "#9fd8ff", r=0.03, rough=0.1, mat={"emissive": "#ffe9a8", "emissive_strength": 2}),
    rb(f"rahmen_h_{i}", [0.86, 0.07, 0.1], [0, 0, 0.02], "#ffffff", r=0.02),
    rb(f"rahmen_v_{i}", [0.07, 0.86, 0.1], [0, 0, 0.02], "#ffffff", r=0.02),
    rb(f"bank_{i}", [0.95, 0.08, 0.18], [0, -0.45, 0.06], "#ffffff", r=0.02)], position=[x, 1.75, 1.62])
add("haus", "Cartoon-Häuschen 4 × 3,2 m, Dach rot, Tür vorne (+Z), zwei Fenster, Schornstein. Farbe über material_override nur einheitlich – besser Kopie anpassen.", group("haus", [
    rb("waende", [4.0, 3.0, 3.2], [0, 1.5, 0], "#f6d9a8", r=0.12, rough=0.85),
    rb("sockel", [4.15, 0.35, 3.35], [0, 0.17, 0], "#b9a58c", r=0.06),
    {"id": "dach", "type": "extrude", "profile": [[-2.45, 0], [2.45, 0], [0, 1.7]], "height": 3.7, "rotation": [-90, 0, 0], "position": [0, 2.98, 1.85],
     "material": {"color": "#c0392b", "roughness": 0.6, "pattern": {"type": "stripes", "color2": "#a93226", "scale": 4, "bump": 0.4}}},
    rb("schornstein", [0.5, 1.3, 0.5], [1.15, 4.0, -0.5], "#a0522d", r=0.05, mat={"pattern": {"type": "bricks", "color2": "#7d3c1f", "scale": 2.5, "bump": 0.5}}),
    rb("tuer", [0.95, 1.8, 0.1], [0, 0.95, 1.62], "#7b4a2a", r=0.06),
    sph("knauf", 0.06, [0.33, 0.95, 1.7], "#f1c40f", mat={"metallic": 1, "roughness": 0.3}),
    win(1, -1.25), win(2, 1.25)]))

# ---------- Auto ----------
wheel = lambda i, x, z: group(f"rad_{i}", [
    cyl(f"reifen_{i}", 0.38, 0.32, [0, 0, 0], "#222222", rough=0.9, rotation=[0, 0, 90]),
    cyl(f"felge_{i}", 0.19, 0.34, [0, 0, 0], "#dddddd", rotation=[0, 0, 90], mat={"metallic": 1, "roughness": 0.3})], position=[x, 0.38, z])
add("auto", "Cartoon-Auto 2 × 3,6 m, rot lackiert, fährt Richtung +Z (Scheinwerfer vorne). Für Fahrten: animate.path mit face: true.", group("auto", [
    rb("karosserie", [2.0, 0.65, 3.6], [0, 0.7, 0], "#d62828", r=0.28, rough=0.35, mat={"clearcoat": 0.25}),
    rb("kabine", [1.7, 0.62, 1.9], [0, 1.25, -0.25], "#d62828", r=0.3, rough=0.35, mat={"clearcoat": 0.25}),
    rb("scheibe_vorne", [1.5, 0.48, 0.06], [0, 1.27, 0.71], "#a8dcff", r=0.05, rough=0.05, rotation=[-25, 0, 0]),
    rb("scheiben_seite", [1.74, 0.42, 1.5], [0, 1.28, -0.25], "#a8dcff", r=0.15, rough=0.05),
    wheel(1, -0.95, 1.15), wheel(2, 0.95, 1.15), wheel(3, -0.95, -1.15), wheel(4, 0.95, -1.15),
    sph("licht_l", 0.13, [-0.65, 0.75, 1.78], "#fff6c8", mat={"emissive": "#fff2b0", "emissive_strength": 20}),
    sph("licht_r", 0.13, [0.65, 0.75, 1.78], "#fff6c8", mat={"emissive": "#fff2b0", "emissive_strength": 20}),
    rb("stossstange", [2.05, 0.18, 0.2], [0, 0.45, 1.8], "#cccccc", r=0.06, rough=0.3, mat={"metallic": 1})]))

# ---------- Laterne ----------
add("laterne", "Straßenlaterne 3,6 m mit warmem Licht (point_light) – nachts gemütlich.", group("laterne", [
    {"id": "mast", "type": "lathe", "profile": [[0.14, 0], [0.14, 0.18], [0.07, 0.25], [0.05, 3.2], [0.09, 3.3], [0, 3.3]], "material": {"color": "#2f3640", "metallic": 0.8, "roughness": 0.4}},
    {"id": "kopf", "type": "lathe", "profile": [[0, 3.75], [0.28, 3.55], [0.22, 3.3], [0, 3.3]], "material": {"color": "#2f3640", "metallic": 0.8, "roughness": 0.4}},
    sph("leuchte", 0.17, [0, 3.38, 0], "#fff3c4", mat={"emissive": "#ffd98a", "emissive_strength": 40}),
    {"id": "licht", "type": "point_light", "color": "#ffcf87", "intensity": 9000, "range": 16, "position": [0, 3.2, 0], "shadows": True}]))

# ---------- Bank ----------
planks = [rb(f"sitz_{i}", [1.8, 0.06, 0.13], [0, 0.46, -0.2 + i * 0.15], "#b5651d", r=0.02, rough=0.8) for i in range(3)]
planks += [rb(f"lehne_{i}", [1.8, 0.13, 0.05], [0, 0.7 + i * 0.18, -0.32], "#b5651d", r=0.02, rough=0.8, rotation=[-10, 0, 0]) for i in range(2)]
legs = [rb(f"bein_{i}", [0.07, 0.46, 0.5], [x, 0.23, -0.05], "#2f3640", r=0.02, mat={"metallic": 0.8}) for i, x in enumerate([-0.75, 0.75])]
add("bank", "Parkbank 1,8 m breit, Sitzfläche vorne (+Z), Sitzhöhe 0,46 m.", group("bank", planks + legs))

# ---------- Zaun ----------
add("zaun", "Holzzaun-Stück 4 m lang (von x = −2 bis +2), mit repeat aneinanderreihen (offset [4, 0, 0]).", group("zaun", [
    {**rb("pfosten", [0.1, 1.0, 0.1], [-2.0, 0.5, 0], "#f5f0e6", r=0.03), "repeat": {"count": 6, "offset": [0.8, 0, 0]}},
    rb("latte_oben", [4.1, 0.09, 0.05], [0, 0.78, 0.06], "#f5f0e6", r=0.02),
    rb("latte_unten", [4.1, 0.09, 0.05], [0, 0.35, 0.06], "#f5f0e6", r=0.02)]))

# ---------- Tisch & Stuhl ----------
add("tisch", "Holztisch 1,4 × 0,9 m, Höhe 0,76 m.", group("tisch", [
    rb("platte", [1.4, 0.07, 0.9], [0, 0.74, 0], "#c68642", r=0.03, rough=0.6, mat={"pattern": {"type": "wood", "color2": "#9c6430", "scale": 2}})] +
    [cyl(f"bein_{i}", 0.04, 0.72, [x, 0.36, z], "#8b5a2b") for i, (x, z) in enumerate([(-0.62, -0.37), (0.62, -0.37), (-0.62, 0.37), (0.62, 0.37)])]))
add("stuhl", "Holzstuhl, Sitzhöhe 0,46 m, Lehne hinten (−Z).", group("stuhl", [
    rb("sitz", [0.46, 0.06, 0.46], [0, 0.46, 0], "#c68642", r=0.03),
    rb("lehne", [0.46, 0.4, 0.05], [0, 0.72, -0.21], "#c68642", r=0.03)] +
    [cyl(f"bein_{i}", 0.025, 0.45, [x, 0.225, z], "#8b5a2b") for i, (x, z) in enumerate([(-0.19, -0.19), (0.19, -0.19), (-0.19, 0.19), (0.19, 0.19)])]))

# ---------- Wolke ----------
add("wolke", "Flauschige Cartoon-Wolke ca. 4 m breit – hoch oben platzieren, gerne mit animate.position langsam ziehen lassen.", group("wolke", [
    sph(f"puff_{i}", r, p, "#ffffff", rough=0.95, subdivide=1, deform=[{"type": "noise", "amount": r * 0.08, "scale": 1.5 / r, "seed": i}])
    for i, (r, p) in enumerate([(1.0, [0, 1.0, 0]), (0.75, [-1.1, 0.75, 0.1]), (0.8, [1.05, 0.8, -0.1]), (0.6, [-1.9, 0.55, 0]), (0.55, [1.85, 0.55, 0.1]), (0.7, [0.3, 0.6, 0.6])])]))

# ---------- Brunnen ----------
add("brunnen", "Steinbrunnen ⌀ 3,2 m mit sprudelndem Wasser (Partikel).", group("brunnen", [
    {"id": "becken", "type": "lathe", "profile": [[0, 0], [1.55, 0], [1.6, 0.1], [1.6, 0.55], [1.42, 0.6], [1.36, 0.18], [0, 0.18]],
     "material": {"color": "#b8b0a2", "roughness": 0.85, "pattern": {"type": "noise", "color2": "#9a9184", "scale": 3, "bump": 0.5}}},
    cyl("wasser", 1.38, 0.04, [0, 0.45, 0], "#3a8fd1", rough=0.04, mat={"clearcoat": 1}),
    {"id": "saeule", "type": "lathe", "profile": [[0.25, 0], [0.18, 0.4], [0.12, 1.1], [0.35, 1.2], [0.32, 1.3], [0, 1.3]], "material": {"color": "#b8b0a2", "roughness": 0.8}},
    {"id": "fontaene", "type": "particles", "preset": "fountain", "position": [0, 1.3, 0]}]))

# ---------- Blume, Busch, Felsen ----------
petals = [sph(f"blatt_{i}", 0.045, [0.06 * __import__('math').cos(i * 1.047), 0.52, 0.06 * __import__('math').sin(i * 1.047)], "#ff6fae", scale=[1, 0.5, 1]) for i in range(6)]
add("blume", "Blume 0,55 m (rosa Blüte, gelbe Mitte). Mit material_override andere Blütenfarbe.", group("blume", [
    {"id": "stiel", "type": "tube", "path": [[0, 0, 0], [0.03, 0.25, 0], [0, 0.5, 0]], "radius": 0.012, "material": {"color": "#3e8e41"}},
    sph("mitte", 0.035, [0, 0.53, 0], "#ffd23f")] + petals))
add("busch", "Runder Busch ca. 1,6 m breit.", group("busch", [
    sph(f"teil_{i}", r, p, c, rough=0.9, subdivide=1, deform=[{"type": "noise", "amount": r * 0.15, "scale": 2.5 / r, "seed": i + 3}])
    for i, (r, p, c) in enumerate([(0.6, [0, 0.55, 0], "#3f8f3a"), (0.45, [-0.5, 0.4, 0.1], "#4ca444"), (0.45, [0.5, 0.42, -0.1], "#378232"), (0.4, [0.1, 0.38, 0.45], "#4ca444")])]))
add("felsen", "Felsbrocken ca. 1,4 m mit kleinem Stein daneben (Low-Poly).", group("felsen", [
    sph("gross", 0.7, [0, 0.5, 0], "#8d8a85", subdivide=2, deform=[{"type": "noise", "amount": 0.22, "scale": 1.6, "octaves": 4, "seed": 2}], mat={"flat": True}, scale=[1.2, 0.8, 1]),
    sph("klein", 0.3, [0.95, 0.2, 0.3], "#97938d", subdivide=1, deform=[{"type": "noise", "amount": 0.1, "scale": 3, "seed": 5}], mat={"flat": True})]))

out = os.path.join(os.path.dirname(__file__), "..", "apps", "studio", "assets", "bibliothek.json")
json.dump(assets, open(out, "w"), ensure_ascii=False, indent=1)
print(f"{len(assets)} Objekte → {os.path.normpath(out)}")
