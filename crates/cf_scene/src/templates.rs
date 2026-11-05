//! Szenen-Vorlagen: fertig ausgeleuchtete Umgebungen als Startpunkt, damit jede neue
//! Szene sofort gut aussieht (`new_scene` mit `template`).

use serde_json::{json, Value};

/// Namen und Kurzbeschreibung aller Vorlagen.
pub const TEMPLATES: &[(&str, &str)] = &[
    ("wiese", "Sonniger Tag auf einer Hügelwiese mit Bäumen und Wolken"),
    ("dorf", "Dorfplatz mit Häuschen, Brunnen, Laternen, Bäumen (Bibliotheks-Objekte)"),
    ("wald", "Dichter Wald mit Dunst und Lichtstrahlen am Morgen"),
    ("strand", "Sandstrand mit Meer, Wellen und Palmen"),
    ("nacht", "Nacht mit Sternen, Mond, Lagerfeuer und Laterne"),
    ("studio", "Neutrales Foto-Studio mit Hohlkehle und Drei-Punkt-Licht (Figuren, Produkte)"),
    ("weltraum", "Weltraum mit Sternen, Planet und Mond"),
    ("zimmer", "Gemütliches Zimmer mit Fenster, Tisch, Stühlen und Lampe"),
    ("cartoon_2d", "2D-Zeichentrick: 2D-Kamera, gemalter Hintergrund mit Himmel und Hügeln"),
];

fn base(name: &str, desc: &str) -> Value {
    json!({"name": name, "description": desc, "duration": 6, "fps": 30, "objects": []})
}

/// Liefert die Vorlage als Szenen-JSON (oder None bei unbekanntem Namen).
pub fn template(name: &str) -> Option<Value> {
    let mut s = match name {
        "wiese" => {
            let mut s = base("Wiese", "Sonniger Tag auf einer Hügelwiese.");
            s["environment"] = json!({"sun": {"azimuth": 215, "elevation": 38, "color": "#fff4e0"}, "fog": {"visibility": 600, "color": "#d4e4f2"}, "grade": {"saturation": 1.1, "vignette": 0.2}});
            s["camera"] = json!({"position": [0, 2.2, 14], "look_at": [0, 1.2, 0], "fov": 40});
            s["objects"] = json!([
                {"id": "wiese", "type": "terrain", "size": [240, 240], "resolution": 192, "height": 6, "style": "hills", "feature_size": 50, "flat_center_radius": 16, "material": {"color": "#79c35b", "pattern": {"type": "noise", "color2": "#68b04c", "scale": 0.4, "bump": 0.2}}},
                {"id": "baum_1", "type": "tree", "kind": "oak", "height": 6, "position": [-9, 0, -6], "wind": 0.3, "seed": 1},
                {"id": "baum_2", "type": "tree", "kind": "oak", "height": 5, "position": [10, 0, -9], "wind": 0.3, "seed": 2},
                {"id": "baeume", "type": "scatter", "count": 40, "area": [200, 120], "position": [0, 0, -80], "seed": 3, "on_terrain": "wiese", "shape": {"type": "tree", "kind": "oak", "height": 7, "wind": 0.2}},
                {"id": "wolke_1", "type": "instance", "asset": "wolke", "position": [-12, 16, -40], "scale": 2.5},
                {"id": "wolke_2", "type": "instance", "asset": "wolke", "position": [18, 20, -60], "scale": 3}
            ]);
            s
        }
        "dorf" => {
            let mut s = base("Dorf", "Dorfplatz am Nachmittag.");
            s["environment"] = json!({"sun": {"azimuth": 230, "elevation": 28, "color": "#ffe8c8"}, "grade": {"saturation": 1.1, "vignette": 0.25}});
            s["camera"] = json!({"position": [2, 3.5, 15], "look_at": [0, 1.5, -2], "fov": 42});
            s["objects"] = json!([
                {"id": "boden", "type": "plane", "size": [120, 120], "material": {"color": "#86c46a"}},
                {"id": "platz", "type": "cylinder", "radius": 7, "height": 0.04, "position": [0, 0.02, -3], "material": {"color": "#cdbb9a", "pattern": {"type": "bricks", "color2": "#b5a383", "scale": 1.5, "bump": 0.4}}},
                {"id": "brunnen", "type": "instance", "asset": "brunnen", "position": [0, 0, -3]},
                {"id": "haus_1", "type": "instance", "asset": "haus", "position": [-8, 0, -9], "rotation": [0, 20, 0]},
                {"id": "haus_2", "type": "instance", "asset": "haus", "position": [8, 0, -10], "rotation": [0, -25, 0]},
                {"id": "haus_3", "type": "instance", "asset": "haus", "position": [0, 0, -14]},
                {"id": "laterne_1", "type": "instance", "asset": "laterne", "position": [-4, 0, 1]},
                {"id": "laterne_2", "type": "instance", "asset": "laterne", "position": [4.5, 0, 1]},
                {"id": "bank_1", "type": "instance", "asset": "bank", "position": [-5.5, 0, -2], "rotation": [0, 70, 0]},
                {"id": "baum_1", "type": "tree", "kind": "oak", "height": 6, "position": [-12, 0, -2], "wind": 0.25, "seed": 4},
                {"id": "baum_2", "type": "tree", "kind": "birch", "height": 6, "position": [12, 0, -3], "wind": 0.25, "seed": 5},
                {"id": "busch_1", "type": "instance", "asset": "busch", "position": [6, 0, -6]},
                {"id": "wolke_1", "type": "instance", "asset": "wolke", "position": [-10, 18, -50], "scale": 3}
            ]);
            s
        }
        "wald" => {
            let mut s = base("Wald", "Morgens im Wald, Licht fällt durch die Bäume.");
            s["environment"] = json!({"sun": {"azimuth": 120, "elevation": 14, "color": "#ffe0b0"}, "fog": {"visibility": 120, "color": "#cfd9c8"}, "light_rays": 0.6, "ao": true, "grade": {"saturation": 1.05, "vignette": 0.35}});
            s["camera"] = json!({"position": [0, 1.7, 4], "look_at": [0, 2.2, -8], "fov": 50});
            s["objects"] = json!([
                {"id": "waldboden", "type": "terrain", "size": [200, 200], "resolution": 160, "height": 3, "style": "hills", "feature_size": 25, "flat_center_radius": 8, "material": {"color": "#5b7d3a", "pattern": {"type": "noise", "color2": "#4a6a2f", "scale": 0.6, "bump": 0.4}}},
                {"id": "tannen", "type": "scatter", "count": 220, "area": [140, 140], "seed": 7, "on_terrain": "waldboden", "keep_clear_radius": 9, "shape": {"type": "tree", "kind": "pine", "height": 11, "wind": 0.15}},
                {"id": "laubbaeume", "type": "scatter", "count": 60, "area": [100, 100], "seed": 8, "on_terrain": "waldboden", "keep_clear_radius": 9, "shape": {"type": "tree", "kind": "oak", "height": 8, "wind": 0.2}},
                {"id": "felsen_1", "type": "instance", "asset": "felsen", "position": [3, 0, -3]},
                {"id": "staub", "type": "particles", "preset": "dust", "position": [0, 2, 0], "area": [16, 4, 16]}
            ]);
            s
        }
        "strand" => {
            let mut s = base("Strand", "Sonniger Sandstrand am Meer.");
            s["environment"] = json!({"sun": {"azimuth": 200, "elevation": 40, "color": "#fff3dc"}, "grade": {"saturation": 1.15, "vignette": 0.2}});
            s["camera"] = json!({"position": [0, 3.5, 12], "look_at": [0, 0.8, -25], "fov": 45});
            s["objects"] = json!([
                {"id": "sand", "type": "plane", "size": [300, 80], "position": [0, 0, 28], "material": {"color": "#f0d7a0", "pattern": {"type": "noise", "color2": "#e3c588", "scale": 1.2, "bump": 0.3}}},
                {"id": "nasser_sand", "type": "plane", "size": [300, 4], "position": [0, 0.005, -10], "material": {"color": "#d9bd86", "roughness": 0.3}},
                {"id": "meer", "type": "water", "size": [600, 400], "position": [0, 0.06, -212], "wave_height": 0.3, "wavelength": 6, "resolution": 192, "color": "#0e5f8f"},
                {"id": "palme_1", "type": "tree", "kind": "palm", "height": 7, "position": [-7, 0, 2], "wind": 0.5, "seed": 2},
                {"id": "palme_2", "type": "tree", "kind": "palm", "height": 6, "position": [8, 0, 4], "wind": 0.5, "seed": 5},
                {"id": "wolke_1", "type": "instance", "asset": "wolke", "position": [15, 22, -80], "scale": 3}
            ]);
            s
        }
        "nacht" => {
            let mut s = base("Nacht", "Sternklare Nacht mit Lagerfeuer.");
            s["environment"] = json!({"sun": {"azimuth": 220, "elevation": -10}, "stars": 1.4, "moon": {"azimuth": 150, "elevation": 32, "size": 2}, "ambient": 0.5, "bloom": 0.3, "grade": {"vignette": 0.4}});
            s["camera"] = json!({"position": [0, 2, 10], "look_at": [0, 1.5, 0], "fov": 45});
            s["objects"] = json!([
                {"id": "wiese", "type": "terrain", "size": [200, 200], "resolution": 128, "height": 4, "style": "hills", "feature_size": 40, "flat_center_radius": 12, "material": {"color": "#3d5a33"}},
                {"id": "tannen", "type": "scatter", "count": 80, "area": [150, 100], "position": [0, 0, -60], "seed": 2, "on_terrain": "wiese", "shape": {"type": "tree", "kind": "pine", "height": 9}},
                {"id": "feuer", "type": "particles", "preset": "fire", "position": [0, 0.15, 0]},
                {"id": "funken", "type": "particles", "preset": "sparks", "position": [0, 0.4, 0]},
                {"id": "feuerlicht", "type": "point_light", "color": "#ff9a4a", "intensity": 40000, "range": 25, "position": [0, 1, 0], "shadows": true, "animate": {"flicker": 0.4}},
                {"id": "holz", "type": "cylinder", "radius": 0.08, "height": 1, "position": [0, 0.08, 0], "rotation": [0, 30, 90], "repeat": {"count": 3, "rotation": [0, 60, 0]}, "material": {"color": "#5a3a1e"}},
                {"id": "laterne_1", "type": "instance", "asset": "laterne", "position": [-6, 0, -4]}
            ]);
            s
        }
        "studio" => {
            let mut s = base("Studio", "Neutrales Studio mit Hohlkehle und Drei-Punkt-Licht.");
            s["environment"] = json!({"sky": {"type": "color", "color": "#eef1f6"}, "sun": {"azimuth": 160, "elevation": 55, "intensity": 0.8}, "ambient": 1.2, "exposure": 0.6, "ao": true, "bloom": 0.1});
            s["camera"] = json!({"position": [0, 1.4, 6], "look_at": [0, 1, 0], "fov": 35});
            s["objects"] = json!([
                {"id": "boden", "type": "plane", "size": [30, 30], "material": {"color": "#f2f3f6", "roughness": 0.5}},
                {"id": "hohlkehle", "type": "lathe", "profile": [[8, 0], [7.2, 0.15], [6.3, 0.8], [6, 1.6], [6, 9]], "angle": 140, "rotation": [0, 160, 0], "position": [0, 0, 3], "material": {"color": "#f2f3f6", "roughness": 0.5, "double_sided": true}},
                {"id": "hauptlicht", "type": "point_light", "color": "#fff1dc", "intensity": 120000, "range": 30, "position": [-3, 4, 4], "shadows": true},
                {"id": "fuelllicht", "type": "point_light", "color": "#dce8ff", "intensity": 40000, "range": 30, "position": [4, 2.5, 3]},
                {"id": "kantenlicht", "type": "point_light", "color": "#ffffff", "intensity": 70000, "range": 30, "position": [0, 4, -3]}
            ]);
            s
        }
        "weltraum" => {
            let mut s = base("Weltraum", "Im All: Planet, Mond und Sterne.");
            s["environment"] = json!({"sky": {"type": "color", "color": "#02030a"}, "stars": 2.5, "sun": {"azimuth": 250, "elevation": 10, "intensity": 1.2}, "ambient": 0.05, "bloom": 0.35});
            s["camera"] = json!({"position": [0, 0, 30], "look_at": [0, 0, 0], "fov": 40});
            s["objects"] = json!([
                {"id": "planet", "type": "sphere", "radius": 9, "position": [-4, -2, -10], "rotation": [10, 0, 15], "animate": {"spin": [0, 3, 0]}, "material": {"color": "#3a7bd5", "roughness": 0.7, "pattern": {"type": "marble", "color2": "#e8f1ff", "scale": 0.15, "bump": 0.2}}},
                {"id": "ring", "type": "torus", "radius": 14, "thickness": 0.6, "position": [-4, -2, -10], "rotation": [72, 0, 15], "scale": [1, 1, 0.08], "material": {"color": "#d8c7a0", "roughness": 0.6, "opacity": 0.8}},
                {"id": "mond", "type": "sphere", "radius": 2, "position": [12, 5, 0], "material": {"color": "#c9c9c9", "roughness": 0.9, "pattern": {"type": "noise", "color2": "#9a9a9a", "scale": 0.8, "bump": 0.8}}}
            ]);
            s
        }
        "zimmer" => {
            let mut s = base("Zimmer", "Gemütliches Zimmer am Abend.");
            s["environment"] = json!({"sky": {"type": "color", "color": "#2a3550"}, "sun": {"azimuth": 160, "elevation": 35, "intensity": 0.5, "color": "#ffd8a8", "shadows": false}, "ambient": 1.0, "exposure": 1.0, "ao": true, "bloom": 0.2, "grade": {"vignette": 0.3}});
            s["camera"] = json!({"position": [0, 1.6, 4.5], "look_at": [0, 1.1, -1], "fov": 50});
            s["objects"] = json!([
                {"id": "boden", "type": "box", "size": [7, 0.1, 7], "position": [0, -0.05, 0], "material": {"color": "#b07a4a", "pattern": {"type": "wood", "color2": "#8d5d34", "scale": 1.5, "bump": 0.3}}},
                {"id": "wand_hinten", "type": "extrude", "profile": [[-3.5, 0], [3.5, 0], [3.5, 0.12], [-3.5, 0.12]], "holes": [[[0.8, 0.0], [2.4, 0.0], [2.4, 0.12], [0.8, 0.12]]], "height": 3, "position": [0, 0, -3.5], "material": {"color": "#e8dcc8"}},
                {"id": "wand_links", "type": "box", "size": [0.12, 3, 7], "position": [-3.5, 1.5, 0], "material": {"color": "#e8dcc8"}},
                {"id": "wand_rechts", "type": "box", "size": [0.12, 3, 7], "position": [3.5, 1.5, 0], "material": {"color": "#e8dcc8"}},
                {"id": "fenster", "type": "plane", "size": [1.6, 1.2], "position": [1.6, 1.7, -3.42], "rotation": [90, 0, 0], "material": {"color": "#ffd59a", "emissive": "#ffcf8a", "emissive_strength": 8, "unlit": true}},
                {"id": "tisch", "type": "instance", "asset": "tisch", "position": [0, 0, -0.5]},
                {"id": "stuhl_1", "type": "instance", "asset": "stuhl", "position": [-0.9, 0, -0.5], "rotation": [0, 90, 0]},
                {"id": "stuhl_2", "type": "instance", "asset": "stuhl", "position": [0.9, 0, -0.5], "rotation": [0, -90, 0]},
                {"id": "lampe", "type": "point_light", "color": "#ffcf87", "intensity": 160000, "range": 14, "position": [0, 2.5, -0.5], "shadows": true},
                {"id": "fuelllicht", "type": "point_light", "color": "#cfdcff", "intensity": 60000, "range": 14, "position": [0, 2.2, 3.5]},
                {"id": "lampenschirm", "type": "lathe", "profile": [[0.35, 2.55], [0.15, 2.85], [0, 2.86]], "material": {"color": "#f4e1c1", "emissive": "#ffd59a", "emissive_strength": 3, "double_sided": true}}
            ]);
            s
        }
        "cartoon_2d" => {
            let mut s = base("Cartoon 2D", "2D-Zeichentrick mit gemaltem Hintergrund.");
            s["environment"] = json!({"sky": {"type": "color", "color": "#9ad8ff"}, "bloom": 0});
            s["camera"] = json!({"position": [0, 2.5, 20], "look_at": [0, 2.5, 0], "orthographic": 7});
            s["objects"] = json!([
                {"id": "hintergrund", "type": "svg", "width": 18, "position": [0, 2.5, -3], "svg": "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1600 700'><defs><linearGradient id='h' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='#7cc8ff'/><stop offset='1' stop-color='#d8f0ff'/></linearGradient></defs><rect width='1600' height='700' fill='url(#h)'/><circle cx='1320' cy='150' r='80' fill='#ffe066'/><path d='M0 500 Q 300 380 600 480 T 1200 460 T 1600 470 L1600 700 L0 700 Z' fill='#6cc46a'/><path d='M0 600 Q 400 520 800 600 T 1600 590 L1600 700 L0 700 Z' fill='#4aa04a'/></svg>"}
            ]);
            s
        }
        _ => return None,
    };
    s["version"] = json!(1);
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_templates_are_valid_scenes() {
        for (name, _) in TEMPLATES {
            let v = template(name).unwrap();
            let s = crate::Scene::from_json(&v.to_string());
            assert!(s.is_ok(), "{name}: {:?}", s.err());
        }
        assert!(template("gibtsnicht").is_none());
    }
}
