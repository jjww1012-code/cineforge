//! Licht-Stimmungen per Stichwort: fertige Kombinationen aus Sonnenstand, Lichtfarbe, Dunst,
//! Sternen/Mond und Farbkorrektur – wie ein Kameramann sie für eine Tageszeit wählen würde.
//! Wird als Merge-Patch auf `environment` angewandt (`update_scene_settings` mit `lighting`).

use serde_json::{json, Value};

pub const PRESETS: &[(&str, &str)] = &[
    ("morning", "Morgen: tiefe, warme Sonne, leichter Dunst, Lichtstrahlen"),
    ("noon", "Mittag: hohe, klare Sonne, kräftige Farben"),
    ("golden_hour", "Goldene Stunde: tiefe, goldene Sonne, lange Schatten"),
    ("sunset", "Sonnenuntergang: orange-rote Sonne am Horizont"),
    ("blue_hour", "Blaue Stunde: nach Sonnenuntergang, kühles Licht, erste Sterne"),
    ("night", "Nacht: Mondlicht, Sterne"),
    ("overcast", "Bewölkt: weiches, graues Licht ohne harte Schatten"),
    ("magic", "Magisch/märchenhaft: rosa-violetter Dunst, Lichtstrahlen, Glühen"),
    ("spooky", "Unheimlich: dichter, kalter Nebel, großer Mond"),
];

/// Umgebungs-Patch für eine Licht-Stimmung (oder None bei unbekanntem Namen).
pub fn preset(name: &str) -> Option<Value> {
    Some(match name {
        "morning" => json!({"sun": {"azimuth": 100, "elevation": 12, "color": "#ffe4c4", "intensity": 1.0}, "fog": {"visibility": 300, "color": "#dfe8f0"},
            "light_rays": 0.3, "stars": 0, "moon": null, "exposure": 0, "grade": {"saturation": 1.05, "vignette": 0.2}}),
        "noon" => json!({"sun": {"azimuth": 180, "elevation": 60, "color": "#fffaf0", "intensity": 1.0}, "fog": {"visibility": 800, "color": "#dbe6f2"},
            "light_rays": 0, "stars": 0, "moon": null, "exposure": 0, "grade": {"saturation": 1.1, "vignette": 0.15}}),
        "golden_hour" => json!({"sun": {"azimuth": 250, "elevation": 8, "color": "#ffc98a", "intensity": 1.0}, "fog": {"visibility": 400, "color": "#f3d9b8"},
            "light_rays": 0.4, "stars": 0, "moon": null, "exposure": 0.2, "grade": {"saturation": 1.15, "vignette": 0.3}}),
        "sunset" => json!({"sun": {"azimuth": 260, "elevation": 4, "color": "#ffb47a", "intensity": 1.0}, "fog": {"visibility": 380, "color": "#f2c4a0"},
            "light_rays": 0.45, "stars": 0, "moon": null, "ambient": 1.2, "exposure": 0.7, "grade": {"saturation": 1.08, "vignette": 0.3}}),
        "blue_hour" => json!({"sun": {"azimuth": 270, "elevation": -4, "color": "#9fb4ff", "intensity": 0.6}, "fog": {"visibility": 250, "color": "#5a6a90"},
            "light_rays": 0, "stars": 0.6, "moon": null, "ambient": 0.6, "exposure": 0, "grade": {"saturation": 1.0, "vignette": 0.35}}),
        "night" => json!({"sun": {"azimuth": 220, "elevation": -12}, "fog": null, "light_rays": 0, "stars": 1.4,
            "moon": {"azimuth": 150, "elevation": 30, "size": 1.5, "intensity": 1.6}, "ambient": 0.7, "bloom": 0.25, "exposure": 0, "grade": {"saturation": 1.0, "vignette": 0.4}}),
        "overcast" => json!({"sun": {"azimuth": 180, "elevation": 40, "color": "#e8edf2", "intensity": 0.6}, "fog": {"visibility": 220, "color": "#c9d2db"},
            "light_rays": 0, "stars": 0, "moon": null, "ambient": 1.2, "exposure": 0, "grade": {"saturation": 0.9, "vignette": 0.2}}),
        "magic" => json!({"sun": {"azimuth": 230, "elevation": 15, "color": "#ffd0f0", "intensity": 1.0}, "fog": {"visibility": 380, "color": "#dccaff"},
            "light_rays": 0.7, "stars": 0, "moon": null, "bloom": 0.35, "exposure": 0.2, "grade": {"saturation": 1.2, "vignette": 0.35}}),
        "spooky" => json!({"sun": {"azimuth": 200, "elevation": -2, "color": "#7fa0c0", "intensity": 0.5}, "fog": {"visibility": 60, "color": "#2c3a48"},
            "light_rays": 0, "stars": 0.3, "moon": {"azimuth": 120, "elevation": 25, "size": 2.2, "color": "#cfe0ff", "intensity": 2.2}, "ambient": 0.9, "exposure": 0.5,
            "grade": {"saturation": 0.8, "vignette": 0.5}}),
        _ => return None,
    })
}

/// Namen aller Stimmungen (für Fehlermeldungen).
pub fn names() -> Vec<&'static str> {
    PRESETS.iter().map(|(n, _)| *n).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_presets_give_valid_scenes() {
        for (name, _) in PRESETS {
            let base = crate::model::Scene::from_json("{}").unwrap();
            let s = crate::ops::update_settings(&base, &json!({"lighting": name})).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(s.validate().is_empty(), "{name}: {:?}", s.validate());
        }
        let base = crate::model::Scene::from_json("{}").unwrap();
        assert!(crate::ops::update_settings(&base, &json!({"lighting": "abend"})).is_err());
        // eigene Angaben gehen vor
        let s = crate::ops::update_settings(&base, &json!({"lighting": "noon", "environment": {"sun": {"elevation": 30}}})).unwrap();
        assert_eq!(s.environment.sun.elevation, 30.0);
        assert_eq!(s.environment.sun.azimuth, 180.0);
    }
}
