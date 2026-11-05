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
    ("schloss", "Märchenschloss mit Türmen und Fahnen auf einem Hügel im goldenen Abendlicht"),
    ("unterwasser", "Unterwasserwelt mit Sand, Korallen, Seegras, Fischen und Lichtstrahlen"),
    ("stadt", "Straße in einer bunten Stadt mit Häusern, Autos, Laternen und Bäumen"),
    ("bauernhof", "Bauernhof mit roter Scheune, Bauernhaus, Feld, Zaun, Kuh, Schwein, Schafen und Hühnern"),
    ("winter", "Verschneite Winterlandschaft mit Tannen, Schneemann, Häuschen und fallendem Schnee"),
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
        "schloss" => {
            let mut s = base("Schloss", "Märchenschloss im Abendlicht.");
            s["environment"] = json!({"sun": {"azimuth": 250, "elevation": 12, "color": "#ffd29a"}, "fog": {"visibility": 400, "color": "#f2d6b8"}, "light_rays": 0.4, "bloom": 0.25, "grade": {"saturation": 1.15, "vignette": 0.3}});
            s["camera"] = json!({"position": [0, 3, 34], "look_at": [0, 8, 0], "fov": 40});
            let stone = json!({"color": "#e9e2d6", "roughness": 0.85, "pattern": {"type": "bricks", "color2": "#cfc6b8", "scale": 0.8, "bump": 0.5}});
            let roof = json!({"color": "#3b6fd1", "roughness": 0.5});
            let mut objs = vec![
                json!({"id": "huegel", "type": "terrain", "size": [300, 300], "resolution": 160, "height": 5, "style": "hills", "feature_size": 60, "flat_center_radius": 20, "material": {"color": "#6fb04f"}}),
                json!({"id": "bergfried", "type": "rounded_box", "size": [8, 16, 8], "radius": 0.3, "position": [0, 8, -4], "material": stone}),
                json!({"id": "bergfried_dach", "type": "cone", "radius": 5.2, "height": 7, "position": [0, 19.5, -4], "material": roof}),
                json!({"id": "mauer_vorne", "type": "box", "size": [18, 6, 1.2], "position": [0, 3, 4], "material": stone}),
                json!({"id": "zinnen", "type": "box", "size": [0.8, 0.9, 1.2], "position": [-8.6, 6.45, 4], "repeat": {"count": 12, "offset": [1.56, 0, 0]}, "material": stone}),
                json!({"id": "tor", "type": "rounded_box", "size": [3.2, 4.2, 0.4], "radius": 0.2, "position": [0, 2.1, 4.65], "material": {"color": "#5b3a1e", "roughness": 0.8, "pattern": {"type": "wood", "color2": "#4a2f17", "scale": 1.5}}}),
                json!({"id": "fenster", "type": "rounded_box", "size": [1.2, 2, 0.2], "radius": 0.1, "position": [-2, 12, 0.05], "repeat": {"count": 3, "offset": [2, 0, 0]}, "material": {"color": "#ffd77a", "emissive": "#ffc45a", "emissive_strength": 15}})
            ];
            for (i, (x, z)) in [(-9.0, 4.0), (9.0, 4.0), (-7.0, -9.0), (7.0, -9.0)].iter().enumerate() {
                objs.push(json!({"id": format!("turm_{i}"), "type": "cylinder", "radius": 2.2, "height": 13, "position": [x, 6.5, z], "material": stone}));
                objs.push(json!({"id": format!("turmdach_{i}"), "type": "cone", "radius": 2.8, "height": 5.5, "position": [x, 15.75, z], "material": roof}));
                objs.push(json!({"id": format!("fahne_{i}"), "type": "box", "size": [1.4, 0.8, 0.04], "position": [x + 0.75, 19.3, z], "subdivide": 3, "deform": [{"type": "wave", "amplitude": 0.08, "wavelength": 0.9, "axis": "x"}], "material": {"color": "#e63946", "double_sided": true}}));
                objs.push(json!({"id": format!("fahnenmast_{i}"), "type": "cylinder", "radius": 0.05, "height": 2.2, "position": [x, 19.6, z], "material": {"color": "#555555", "metallic": 1}}));
            }
            objs.push(json!({"id": "baeume", "type": "scatter", "count": 40, "area": [160, 100], "position": [0, 0, 40], "seed": 9, "on_terrain": "huegel", "keep_clear_radius": 30, "shape": {"type": "tree", "kind": "oak", "height": 7, "wind": 0.2}}));
            s["objects"] = Value::Array(objs);
            s
        }
        "unterwasser" => {
            let mut s = base("Unterwasser", "Unterwasserwelt mit Korallen und Fischen.");
            s["environment"] = json!({"sky": {"type": "color", "color": "#0a4f78"}, "sun": {"azimuth": 180, "elevation": 70, "color": "#bfe9ff", "intensity": 0.8}, "fog": {"visibility": 45, "color": "#0e6a96"}, "ambient": 1.1, "light_rays": 0.9, "bloom": 0.3, "grade": {"saturation": 1.15, "vignette": 0.35}});
            s["camera"] = json!({"position": [0, 2, 10], "look_at": [0, 1.5, 0], "fov": 50});
            let mut objs = vec![
                json!({"id": "meeresboden", "type": "terrain", "size": [120, 120], "resolution": 128, "height": 1.5, "style": "dunes", "feature_size": 12, "flat_center_radius": 5, "material": {"color": "#e3c890", "pattern": {"type": "noise", "color2": "#d2b47a", "scale": 1.0, "bump": 0.4}}}),
                json!({"id": "fels_1", "type": "instance", "asset": "felsen", "position": [-4, 0, -2], "scale": 2}),
                json!({"id": "fels_2", "type": "instance", "asset": "felsen", "position": [5, 0, -4], "scale": 2.5}),
                json!({"id": "blasen", "type": "particles", "preset": "magic", "position": [0, 0.5, 0], "area": [14, 1, 10], "color_start": "#e8f8ff", "color_end": "#bfe9ff", "gravity": [0, 0.6, 0]})
            ];
            for (i, (x, z, c)) in [(-2.5, 1.0, "#ff6f61"), (2.0, 0.0, "#ffb84d"), (-5.5, -5.0, "#c86bfa"), (6.0, -1.0, "#ff8fb1"), (0.5, -6.0, "#4dd0e1")].iter().enumerate() {
                objs.push(json!({"id": format!("koralle_{i}"), "type": "sphere", "radius": 0.7, "position": [x, 0.5, z], "subdivide": 2, "deform": [{"type": "noise", "amount": 0.3, "scale": 3.5, "octaves": 3, "seed": i}], "scale": [1, 1.3, 1], "material": {"color": c, "roughness": 0.7}}));
            }
            for i in 0..8 {
                let x = -6.0 + i as f32 * 1.7;
                objs.push(json!({"id": format!("seegras_{i}"), "type": "tube", "radius": 0.08, "radius_end": 0.02, "path": [[x, 0, -3.0 - (i % 3) as f32], [x + 0.3, 1.2, -3.0 - (i % 3) as f32], [x - 0.2, 2.4, -3.0 - (i % 3) as f32]],
                    "material": {"color": "#2f9e44"}, "animate": {"rotation": [{"t": 0, "v": [0, 0, -6]}, {"t": 3, "v": [0, 0, 6]}, {"t": 6, "v": [0, 0, -6]}]}}));
            }
            objs.push(json!({"id": "fisch_1", "type": "animal", "kind": "fish", "size": 0.6, "animate": {"path": {"points": [[-8, 2, 0], [0, 2.5, 2], [8, 2, 0]], "start": 0, "end": 6, "ease": "linear"}}}));
            objs.push(json!({"id": "fisch_2", "type": "animal", "kind": "fish", "size": 0.4, "color": "#4dabf7", "color2": "#fff3bf", "animate": {"path": {"points": [[7, 1.2, -3], [0, 1.6, -2], [-7, 1.4, -4]], "start": 0, "end": 6, "ease": "linear"}}}));
            s["objects"] = Value::Array(objs);
            s
        }
        "stadt" => {
            let mut s = base("Stadt", "Bunte Stadtstraße am Tag.");
            s["environment"] = json!({"sun": {"azimuth": 220, "elevation": 40}, "ao": true, "grade": {"saturation": 1.1, "vignette": 0.2}});
            s["camera"] = json!({"position": [0, 1.7, 14], "look_at": [0, 2.5, -10], "fov": 50});
            let mut objs = vec![
                json!({"id": "strasse", "type": "plane", "size": [8, 120], "position": [0, 0.01, -30], "material": {"color": "#3d3f44", "roughness": 0.9}}),
                json!({"id": "mittellinie", "type": "box", "size": [0.15, 0.01, 2], "position": [0, 0.02, 12], "repeat": {"count": 20, "offset": [0, 0, -4]}, "material": {"color": "#f5f5f5"}}),
                json!({"id": "gehweg_l", "type": "box", "size": [4, 0.2, 120], "position": [-6, 0.1, -30], "material": {"color": "#b8b4ae", "pattern": {"type": "grid", "color2": "#9e9a94", "scale": 1, "bump": 0.3}}}),
                json!({"id": "gehweg_r", "type": "box", "size": [4, 0.2, 120], "position": [6, 0.1, -30], "material": {"color": "#b8b4ae", "pattern": {"type": "grid", "color2": "#9e9a94", "scale": 1, "bump": 0.3}}}),
                json!({"id": "boden", "type": "plane", "size": [200, 200], "position": [0, -0.01, -30], "material": {"color": "#8a8a8a"}}),
                json!({"id": "auto_1", "type": "instance", "asset": "auto", "position": [-1.8, 0, 2], "rotation": [0, 180, 0]}),
                json!({"id": "auto_2", "type": "instance", "asset": "auto", "position": [1.8, 0, -8]})
            ];
            let colors = ["#f4a261", "#e76f51", "#2a9d8f", "#e9c46a", "#8ecae6", "#ffb5a7", "#b5838d", "#90be6d"];
            for i in 0..8 {
                let z = 6.0 - i as f32 * 9.0;
                for (side, x) in [(-1.0f32, -12.5f32), (1.0, 12.5)] {
                    let h = 9.0 + ((i * 7 + if side < 0.0 { 3 } else { 0 }) % 5) as f32 * 2.5;
                    let c = colors[(i + if side < 0.0 { 0 } else { 3 }) % colors.len()];
                    objs.push(json!({"id": format!("haus_{i}_{}", if side < 0.0 { "l" } else { "r" }), "type": "rounded_box", "size": [9, h, 8.5], "radius": 0.15, "position": [x, h / 2.0, z],
                        "material": {"color": c, "roughness": 0.8, "pattern": {"type": "grid", "color2": "#5d7fa3", "scale": 0.7, "bump": 0.2}}}));
                }
                if i % 2 == 0 {
                    objs.push(json!({"id": format!("laterne_{i}"), "type": "instance", "asset": "laterne", "position": [-4.5, 0.2, z]}));
                    objs.push(json!({"id": format!("baum_{i}"), "type": "tree", "kind": "lollipop", "height": 4, "position": [4.8, 0.2, z - 3.0], "leaves": "#5fae4a"}));
                }
            }
            s["objects"] = Value::Array(objs);
            s
        }
        "bauernhof" => {
            let mut s = base("Bauernhof", "Sonniger Nachmittag auf dem Bauernhof.");
            s["environment"] = json!({"sun": {"azimuth": 210, "elevation": 34, "color": "#fff0d8"}, "fog": {"visibility": 500, "color": "#dbe7f0"}, "grade": {"saturation": 1.12, "vignette": 0.22}});
            s["camera"] = json!({"position": [0.5, 2.4, 11], "look_at": [0, 1.0, -1], "fov": 45});
            let red = json!({"color": "#b5352b", "roughness": 0.8, "pattern": {"type": "wood", "color2": "#9c2d25", "scale": 1.0, "bump": 0.3}});
            let white = json!({"color": "#f4efe6", "roughness": 0.7});
            s["objects"] = json!([
                {"id": "wiese", "type": "terrain", "size": [240, 240], "resolution": 160, "height": 5, "style": "hills", "feature_size": 45, "flat_center_radius": 22, "material": {"color": "#86c060", "pattern": {"type": "noise", "color2": "#74ad52", "scale": 0.4, "bump": 0.2}}},
                {"id": "scheune", "type": "group", "position": [-7, 0, -8], "rotation": [0, 15, 0], "children": [
                    {"id": "scheune_waende", "type": "box", "size": [8, 5, 10], "position": [0, 2.5, 0], "material": red},
                    {"id": "scheune_dach", "type": "extrude", "profile": [[-4.6, 0], [4.6, 0], [0, 3.2]], "height": 10.6, "position": [0, 5, 5.3], "rotation": [-90, 0, 0], "material": {"color": "#5b3b2c", "roughness": 0.85}},
                    {"id": "scheune_tor", "type": "box", "size": [3.2, 3.6, 0.12], "position": [0, 1.8, 5.02], "material": red},
                    {"id": "scheune_rahmen", "type": "box", "size": [3.5, 0.2, 0.16], "position": [0, 3.7, 5.05], "material": white},
                    {"id": "scheune_kreuz_1", "type": "box", "size": [4.4, 0.16, 0.14], "position": [0, 1.8, 5.1], "rotation": [0, 0, 48], "material": white},
                    {"id": "scheune_kreuz_2", "type": "box", "size": [4.4, 0.16, 0.14], "position": [0, 1.8, 5.1], "rotation": [0, 0, -48], "material": white},
                    {"id": "heuluke", "type": "box", "size": [1.4, 1.2, 0.12], "position": [0, 5.9, 5.0], "material": white}
                ]},
                {"id": "bauernhaus", "type": "instance", "asset": "haus", "position": [7, 0, -11], "rotation": [0, -20, 0], "scale": 1.3},
                {"id": "feld", "type": "plane", "size": [14, 16], "position": [16, 0.03, -6], "material": {"color": "#9a6a3c", "roughness": 0.95, "pattern": {"type": "stripes", "color2": "#7c522c", "scale": 2.0, "bump": 0.5}}},
                {"id": "zaun_vorne", "type": "instance", "asset": "zaun", "position": [-12, 0, 3], "repeat": {"count": 4, "offset": [4, 0, 0]}},
                {"id": "heuballen", "type": "cylinder", "radius": 0.7, "height": 1.2, "position": [-2, 0.7, -3], "rotation": [0, 0, 90], "repeat": {"count": 2, "offset": [0, 0, 1.6]}, "material": {"color": "#e3c35a", "roughness": 0.95, "pattern": {"type": "noise", "color2": "#c9a640", "scale": 3.0, "bump": 0.6}}},
                {"id": "kuh", "type": "animal", "kind": "cow", "position": [-3, 0, 0], "rotation": [0, 40, 0], "actions": [{"type": "idle", "start": 0, "end": 6}]},
                {"id": "schwein", "type": "animal", "kind": "pig", "position": [2.5, 0, 1], "rotation": [0, -30, 0]},
                {"id": "schaf_1", "type": "animal", "kind": "sheep", "position": [5, 0, -2], "rotation": [0, -60, 0]},
                {"id": "schaf_2", "type": "animal", "kind": "sheep", "position": [6.3, 0, -0.8], "rotation": [0, -100, 0]},
                {"id": "huhn_1", "type": "animal", "kind": "chicken", "position": [0.5, 0, 3], "rotation": [0, 20, 0]},
                {"id": "huhn_2", "type": "animal", "kind": "chicken", "position": [1.2, 0, 3.6], "rotation": [0, -50, 0]},
                {"id": "baum_1", "type": "tree", "kind": "oak", "height": 7, "position": [14, 0, -16], "wind": 0.25, "seed": 6},
                {"id": "baum_2", "type": "tree", "kind": "oak", "height": 6, "position": [-15, 0, -4], "wind": 0.25, "seed": 9},
                {"id": "baeume", "type": "scatter", "count": 30, "area": [220, 80], "position": [0, 0, -90], "seed": 4, "on_terrain": "wiese", "shape": {"type": "tree", "kind": "oak", "height": 8, "wind": 0.2}},
                {"id": "wolke_1", "type": "instance", "asset": "wolke", "position": [-14, 20, -60], "scale": 3}
            ]);
            s
        }
        "winter" => {
            let mut s = base("Winter", "Verschneiter Wintertag.");
            s["environment"] = json!({"sun": {"azimuth": 200, "elevation": 16, "color": "#ffeedd"}, "fog": {"visibility": 260, "color": "#e4ecf5"}, "grade": {"saturation": 1.05, "vignette": 0.25}});
            s["camera"] = json!({"position": [0, 2.2, 12], "look_at": [0, 1.2, 0], "fov": 42});
            let snow = json!({"color": "#f6f9fc", "roughness": 0.65, "pattern": {"type": "noise", "color2": "#e2eaf4", "scale": 0.5, "bump": 0.3}});
            s["objects"] = json!([
                {"id": "schnee", "type": "terrain", "size": [240, 240], "resolution": 160, "height": 5, "style": "hills", "feature_size": 40, "flat_center_radius": 14, "material": snow},
                {"id": "tannen", "type": "scatter", "count": 110, "area": [200, 120], "position": [0, 0, -60], "seed": 5, "on_terrain": "schnee", "shape": {"type": "tree", "kind": "pine", "height": 9, "leaves": "#2c5a48", "wind": 0.1}},
                {"id": "tanne_1", "type": "tree", "kind": "pine", "height": 7, "position": [-7, 0, -4], "leaves": "#2c5a48", "seed": 3},
                {"id": "tanne_2", "type": "tree", "kind": "pine", "height": 5.5, "position": [8, 0, -6], "leaves": "#2c5a48", "seed": 8},
                {"id": "schneemann", "type": "group", "position": [3, 0, -1], "rotation": [0, -20, 0], "children": [
                    {"id": "schneemann_unten", "type": "sphere", "radius": 0.55, "position": [0, 0.5, 0], "material": {"color": "#fbfdff", "roughness": 0.6}},
                    {"id": "schneemann_mitte", "type": "sphere", "radius": 0.4, "position": [0, 1.25, 0], "material": {"color": "#fbfdff", "roughness": 0.6}},
                    {"id": "schneemann_kopf", "type": "sphere", "radius": 0.28, "position": [0, 1.82, 0], "material": {"color": "#fbfdff", "roughness": 0.6}},
                    {"id": "schneemann_nase", "type": "cone", "radius": 0.05, "height": 0.25, "position": [0, 1.82, 0.36], "rotation": [90, 0, 0], "material": {"color": "#f2802a", "roughness": 0.5}},
                    {"id": "schneemann_auge_l", "type": "sphere", "radius": 0.035, "position": [-0.1, 1.9, 0.25], "material": {"color": "#1b1b1b"}},
                    {"id": "schneemann_auge_r", "type": "sphere", "radius": 0.035, "position": [0.1, 1.9, 0.25], "material": {"color": "#1b1b1b"}},
                    {"id": "schneemann_knopf", "type": "sphere", "radius": 0.035, "position": [0, 1.15, 0.39], "repeat": {"count": 3, "offset": [0, 0.13, 0.01]}, "material": {"color": "#1b1b1b"}},
                    {"id": "schneemann_hut", "type": "cylinder", "radius": 0.18, "height": 0.28, "position": [0, 2.15, 0], "material": {"color": "#222226"}},
                    {"id": "schneemann_krempe", "type": "cylinder", "radius": 0.28, "height": 0.03, "position": [0, 2.02, 0], "material": {"color": "#222226"}},
                    {"id": "schneemann_schal", "type": "torus", "radius": 0.24, "thickness": 0.06, "position": [0, 1.58, 0], "material": {"color": "#d62f3a", "roughness": 0.9}},
                    {"id": "schneemann_arm_l", "type": "tube", "radius": 0.025, "path": [[-0.35, 1.3, 0], [-0.7, 1.55, 0], [-0.85, 1.75, 0.05]], "material": {"color": "#5b3a22"}},
                    {"id": "schneemann_arm_r", "type": "tube", "radius": 0.025, "path": [[0.35, 1.3, 0], [0.7, 1.5, 0], [0.9, 1.45, 0.05]], "material": {"color": "#5b3a22"}}
                ]},
                {"id": "huette", "type": "instance", "asset": "haus", "position": [-6, 0, -12], "rotation": [0, 25, 0]},
                {"id": "laterne_1", "type": "instance", "asset": "laterne", "position": [-2.5, 0, 2]},
                {"id": "schneefall", "type": "particles", "preset": "snow", "position": [0, 7, 0], "area": [30, 2, 30]}
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
