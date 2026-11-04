#!/usr/bin/env python3
"""Erzeugt die AXTRM-Endsequenz (Szene 8 Nacht, Zahl, Endkarte) als CineForge-Szenen
für 16:9 und 9:16. Aufruf: python3 scenes/axtrm/generate.py  → schreibt *.json daneben."""
import json, os

MINT, VIOLETT, GOLD, ORANGE, WASSER = "#00D9A3", "#8C7BFF", "#F5C542", "#FF8A2B", "#3BA7FF"
OUT = os.path.dirname(os.path.abspath(__file__))


def k(t, v, ease=None):
    d = {"t": t, "v": v}
    if ease:
        d["ease"] = ease
    return d


def unlit(color, **extra):
    return {"color": color, "unlit": True, **extra}


def text(id, s, size, pos, color="#ffffff", weight="regular", italic=False, align="center", appear=None, spacing=0.0, depth=0.0, rise=0.0):
    o = {"id": id, "type": "text", "text": s, "size": size, "depth": depth, "align": align,
         "weight": weight, "italic": italic, "letter_spacing": spacing, "position": pos,
         "material": unlit(color)}
    if appear:
        t0, t1 = appear
        o["animate"] = {"opacity": [k(0, 0.0), k(t0, 0.0), k(t1, 1.0, "in_out")]}
        if rise:
            o["animate"]["position"] = [k(t0, [pos[0], pos[1] - rise, pos[2]]), k(t1, pos, "out")]
    return o


# ---------------------------------------------------------------------------
# Szene 8: Nacht – Handy auf dem Nachttisch
# ---------------------------------------------------------------------------
def screen_content():
    """Inhalt des Sperrbildschirms in Bildschirm-Koordinaten (x rechts, y oben, Meter)."""
    w = []
    w.append(text("datum", "Dienstag, 14. Oktober", 0.0042, [0, 0.060, 0], "#dfe6ff"))
    w.append(text("uhrzeit", "23:02", 0.026, [0, 0.036, 0], "#ffffff", weight="bold"))
    # Widgets: zwei volle Ringe
    for i, (col, label, x) in enumerate([(MINT, "Ziel erreicht", -0.016), (WASSER, "8 / 8", 0.016)]):
        w.append({"id": f"ring_{i}", "type": "torus", "radius": 0.0068, "thickness": 0.0011,
                  "position": [x, 0.018, 0], "rotation": [90, 0, 0],
                  "material": {"color": col, "emissive": col, "emissive_strength": 2.5}})
        w.append(text(f"ring_label_{i}", label, 0.0026, [x, 0.0075, 0], "#e8ecf5"))
    w.append(text("ring_icon_0", "≈" if False else "13.920", 0.0024, [-0.016, 0.017, 0], "#ffffff", weight="bold"))
    w.append(text("ring_icon_1", "Wasser", 0.0021, [0.016, 0.017, 0], "#ffffff", weight="bold"))
    # Live-Aktivität: Vereinsmeisterschaft
    w.append({"id": "karte", "type": "rounded_box", "size": [0.064, 0.036, 0.0006], "radius": 0.0003,
              "position": [0, -0.022, -0.0004], "material": unlit("#17171d", opacity=0.92)})
    w.append(text("karte_verein", "BOXCLUB EISENHALLE  ·  ✓ Du bist dabei", 0.0024, [-0.029, -0.0085, 0], GOLD, align="left", weight="bold"))
    w.append(text("karte_titel", "Vereinsmeisterschaft", 0.0043, [-0.029, -0.0165, 0], "#ffffff", align="left", weight="bold"))
    w.append(text("karte_ort", "Sa 10:00  ·  Eisenhalle", 0.0027, [-0.029, -0.0225, 0], "#a6a6b4", align="left"))
    w.append(text("karte_beginnt", "Beginnt in", 0.0024, [0.029, -0.0150, 0], "#a6a6b4", align="right"))
    w.append(text("karte_countdown", "2 T 10 h", 0.0062, [0.029, -0.0235, 0], "#ffffff", align="right", weight="bold"))
    w.append(text("karte_dabei", "14 dabei", 0.0026, [-0.029, -0.0335, 0], MINT, align="left", weight="bold"))
    w.append({"id": "karte_route", "type": "rounded_box", "size": [0.014, 0.0055, 0.0006], "radius": 0.0003,
              "position": [0.022, -0.0325, 0], "material": unlit(VIOLETT)})
    w.append(text("karte_route_text", "Route", 0.0026, [0.022, -0.0334, 0.0004], "#ffffff", weight="bold"))
    # Streak-Flamme oben rechts: 47 → 48
    w.append({"id": "flamme", "type": "group", "position": [0.022, 0.068, 0], "children": [
        {"id": "flamme_kern", "type": "sphere", "radius": 0.0022, "material": {"color": ORANGE, "emissive": ORANGE, "emissive_strength": 6}},
        {"id": "flamme_spitze", "type": "cone", "radius": 0.0021, "height": 0.0045, "position": [0, 0.0028, 0],
         "material": {"color": ORANGE, "emissive": ORANGE, "emissive_strength": 6}},
    ], "animate": {"scale": [k(0, 1.0), k(2.0, 1.12, "in_out"), k(4.0, 1.0, "in_out"), k(6.0, 1.12, "in_out"), k(8.0, 1.0, "in_out")]}})
    w.append({**text("streak_47", "47", 0.0042, [0.0275, 0.0665, 0], ORANGE, align="left", weight="bold"),
              "animate": {"visible": [k(0, True), k(3.6, False, "step")]}})
    w.append({**text("streak_48", "48", 0.0042, [0.0275, 0.0665, 0], ORANGE, align="left", weight="bold"),
              "animate": {"visible": [k(0, False), k(3.6, True, "step")],
                          "scale": [k(3.6, 0.3), k(4.1, 1.0, "out_back")]}})
    # Home-Balken
    w.append({"id": "home_balken", "type": "rounded_box", "size": [0.024, 0.0011, 0.0004], "radius": 0.0002,
              "position": [0, -0.072, 0], "material": unlit("#d0d4de")})
    return w


def scene_nacht(portrait):
    if portrait:
        cam = {"position": [0.0, 0.25, 0.115], "look_at": [0, 0, -0.004], "fov": 42,
               "animate": {"position": [k(0, [0.0, 0.25, 0.115]), k(10, [0.0, 0.21, 0.095], "in_out")],
                           "look_at": [k(0, [0, 0, -0.004])]}}
    else:
        cam = {"position": [0.17, 0.17, 0.15], "look_at": [0.0, 0.0, -0.006], "fov": 30,
               "animate": {"position": [k(0, [0.17, 0.17, 0.15]), k(10, [0.12, 0.13, 0.11], "in_out")],
                           "look_at": [k(0, [0.0, 0.0, -0.006])]}}
    objs = [
        {"id": "nachttisch", "type": "rounded_box", "size": [0.9, 0.04, 0.6], "radius": 0.006, "position": [0, -0.02, 0],
         "material": {"color": "#e9cf9f", "roughness": 0.55, "clearcoat": 0.3,
                      "pattern": {"type": "wood", "color2": "#c79b62", "scale": 5, "bump": 0.25, "seed": 3}}},
        {"id": "kaffeering_1", "type": "torus", "radius": 0.034, "thickness": 0.0018, "position": [-0.13, 0.0003, 0.06],
         "material": {"color": "#7a5534", "opacity": 0.55, "roughness": 0.8}},
        {"id": "kaffeering_2", "type": "torus", "radius": 0.033, "thickness": 0.0015, "position": [-0.105, 0.0003, 0.09],
         "material": {"color": "#7a5534", "opacity": 0.35, "roughness": 0.8}},
        {"id": "handy", "type": "group", "rotation": [0, -8, 0], "children": [
            {"id": "gehaeuse", "type": "rounded_box", "size": [0.0752, 0.0083, 0.1604], "radius": 0.0035, "position": [0, 0.00415, 0],
             "material": {"color": "#26262c", "metallic": 0.9, "roughness": 0.3}},
            {"id": "display", "type": "box", "size": [0.0705, 0.0004, 0.1555], "position": [0, 0.0084, 0],
             "material": unlit("#060812")},
            {"id": "bildschirm", "type": "group", "position": [0, 0.0088, 0], "rotation": [-90, 0, 0], "children": screen_content()},
            # Display geht aus: schwarze, glänzende Glasfläche blendet ein (spiegelt danach das Laternenlicht)
            {"id": "glas_aus", "type": "box", "size": [0.0705, 0.0003, 0.1555], "position": [0, 0.0094, 0],
             "material": {"color": "#000000", "roughness": 0.06, "clearcoat": 1.0, "opacity": 0.0},
             "animate": {"opacity": [k(0, 0.0), k(6.4, 0.0), k(7.3, 0.55, "in_out"), k(7.6, 0.55), k(8.1, 1.0, "in")]}},
        ]},
        {"id": "display_licht", "type": "point_light", "position": [0, 0.06, 0.0], "color": "#9db9ff", "intensity": 2.5, "range": 1.0,
         "animate": {"intensity": [k(0, 2.5), k(6.4, 2.5), k(7.3, 1.2, "in_out"), k(8.1, 0.0, "in")]}},
        {"id": "laterne", "type": "point_light", "position": [-0.9, 0.55, -0.7], "color": "#ffb36b", "intensity": 30, "range": 4.0},
        {"id": "laterne_reflex", "type": "point_light", "position": [0.35, 0.45, -0.35], "color": "#ffc285", "intensity": 6, "range": 3.0},
    ]
    return {"version": 1, "name": "AXTRM – Nacht (Szene 8)",
            "description": "23:02, das Handy liegt auf dem Nachttisch. Sperrbildschirm mit Zielen, Event-Countdown und Streak 48; dann geht das Display aus.",
            "duration": 10, "fps": 30,
            "environment": {"sky": {"type": "color", "color": "#000000"}, "sun": {"intensity": 0.0, "shadows": False},
                            "ambient": 0.01, "exposure": 8.5, "bloom": 0.25},
            "camera": cam, "objects": objs}


# ---------------------------------------------------------------------------
# Die Zahl
# ---------------------------------------------------------------------------
def scene_zahl(portrait):
    z = 10.0 if portrait else 6.5
    objs = [
        text("rating", "1246", 1.05, [0, 0.25, 0], "#ffffff", weight="black", italic=True, appear=(0.6, 2.4), rise=0.05),
        text("gestern", "Gestern 1232.", 0.27, [0, -0.42, 0], MINT, italic=True, weight="bold", appear=(3.0, 3.9)),
        text("morgen", "Morgen?", 0.17, [0, -0.85, 0], "#bfc3d1", italic=True, appear=(5.4, 6.3)),
    ]
    return {"version": 1, "name": "AXTRM – Die Zahl", "description": "1246 · Gestern 1232. · Morgen?",
            "duration": 8, "fps": 30,
            "environment": {"sky": {"type": "color", "color": "#000000"}, "sun": {"intensity": 0.0, "shadows": False},
                            "ambient": 0.0, "bloom": 0.2},
            "camera": {"position": [0, 0, z], "look_at": [0, 0, 0], "fov": 35}, "objects": objs}


# ---------------------------------------------------------------------------
# Endkarte: Logo zeichnet sich
# ---------------------------------------------------------------------------
def scene_logo(portrait):
    z = 11.0 if portrait else 7.5
    glow = {"color": MINT, "emissive": MINT, "emissive_strength": 10, "roughness": 0.4}
    # A als Fläche (Profil in XZ mit z = -y, wird um X um 90° gekippt)
    outer = [(-0.62, -0.72), (-0.36, -0.72), (-0.25, -0.40), (0.25, -0.40), (0.36, -0.72), (0.62, -0.72), (0.11, 0.80), (-0.11, 0.80)]
    hole = [(-0.15, -0.20), (0.15, -0.20), (0.0, 0.26)]
    xz = lambda pts: [[x, -y] for (x, y) in pts]
    logo_y = 0.75
    objs = [
        {"id": "a_flaeche", "type": "extrude", "profile": xz(outer), "holes": [xz(hole)], "height": 0.02,
         "position": [0, logo_y, -0.01], "rotation": [90, 0, 0], "material": unlit("#ffffff"),
         "animate": {"opacity": [k(0, 0.0), k(2.2, 0.0), k(3.0, 1.0, "in_out")]}},
        {"id": "a_linie", "type": "tube", "path": [[-0.49, -0.72 + logo_y, 0.02], [0.0, 0.80 + logo_y, 0.02], [0.49, -0.72 + logo_y, 0.02]],
         "radius": 0.022, "segments": 12, "smooth": False, "material": glow,
         "animate": {"draw": [k(0, 0.0), k(0.3, 0.0), k(1.8, 1.0, "in_out")],
                     "opacity": [k(0, 1.0), k(2.6, 1.0), k(3.4, 0.0, "in_out")]}},
        {"id": "schwung", "type": "tube",
         "path": [[-0.95, -0.36 + logo_y, 0.04], [-0.35, -0.30 + logo_y, 0.04], [0.30, -0.16 + logo_y, 0.04], [0.98, 0.14 + logo_y, 0.04]],
         "radius": 0.028, "radius_end": 0.006, "segments": 12, "smooth": True, "material": glow,
         "animate": {"draw": [k(0, 0.0), k(1.4, 0.0), k(2.3, 1.0, "out")]}},
        text("wortmarke", "AXTRM", 0.46, [0, -0.62, 0], "#ffffff", weight="black", italic=True, spacing=0.12, appear=(3.3, 4.1), rise=0.06),
        text("claim", "Dein Tag. Deine Zahl.", 0.17, [0, -1.05, 0], "#d9dbe6", italic=True, appear=(4.9, 5.7)),
        text("store", "Jetzt im App Store  ·  Mit AXTRM Pro", 0.075, [0, -1.95 if portrait else -1.62, 0], "#8b8d9c", appear=(6.4, 7.1)),
    ]
    return {"version": 1, "name": "AXTRM – Endkarte", "description": "Logo zeichnet sich in Mint, füllt sich weiß, Wortmarke, Claim, Store-Hinweis.",
            "duration": 10, "fps": 30,
            "environment": {"sky": {"type": "color", "color": "#000000"}, "sun": {"intensity": 0.0, "shadows": False},
                            "ambient": 0.0, "bloom": 0.45},
            "camera": {"position": [0, -0.25, z], "look_at": [0, -0.25, 0], "fov": 35}, "objects": objs}


for fmt, portrait in [("16x9", False), ("9x16", True)]:
    for name, fn in [("s8_nacht", scene_nacht), ("s9a_zahl", scene_zahl), ("s9b_logo", scene_logo)]:
        with open(os.path.join(OUT, f"{name}_{fmt}.json"), "w") as f:
            json.dump(fn(portrait), f, ensure_ascii=False, indent=1)
print("geschrieben:", sorted(x for x in os.listdir(OUT) if x.endswith(".json")))
