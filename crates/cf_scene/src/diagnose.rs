//! Verständliche Fehlermeldungen, damit Claude Fehler sofort beheben kann:
//! genaue Stelle (`objects[3].material.color`), „Meintest du …?“-Vorschläge
//! und Erkennung von Tippfehlern in Feldnamen, die sonst still ignoriert würden.

use serde_json::Value;

use crate::model::{Kind, Object, Scene};

/// Levenshtein-Abstand (klein und ausreichend für Feldnamen).
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Ähnlichster Kandidat (oder None, wenn nichts ähnlich genug ist).
pub fn suggest<'a>(word: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let w = word.to_lowercase();
    candidates
        .into_iter()
        .map(|c| (distance(&w, &c.to_lowercase()), c))
        .filter(|(d, c)| *d <= (c.len() / 3).max(2))
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

/// Ergänzt eine serde-Fehlermeldung um einen Vorschlag.
/// „unknown field `heigth`, expected one of `size`, `height`“ → „… → Meintest du „height“?“
pub fn explain(msg: &str) -> String {
    let ticks: Vec<&str> = msg.split('`').skip(1).step_by(2).collect();
    let hint = if (msg.contains("unknown field") || msg.contains("unknown variant")) && ticks.len() >= 2 {
        suggest(ticks[0], ticks[1..].iter().copied()).map(|s| format!(" → Meintest du „{s}“?"))
    } else {
        None
    };
    let de = msg
        .replace("unknown field", "unbekanntes Feld")
        .replace("unknown variant", "unbekannter Wert")
        .replace("expected one of", "erlaubt sind")
        .replace("missing field", "fehlendes Pflichtfeld")
        .replace("invalid type", "falscher Typ")
        .replace("expected", "erwartet");
    format!("{de}{}", hint.unwrap_or_default())
}

/// Liest eine Szene mit genauer Fehlerstelle.
pub fn parse_scene(v: Value) -> Result<Scene, String> {
    let input = v.clone();
    let scene: Scene = serde_path_to_error::deserialize(v).map_err(|e| {
        let path = e.path().to_string();
        let msg = explain(&e.inner().to_string());
        if path.is_empty() || path == "." { msg } else { format!("{path}: {msg}") }
    })?;
    let unknown = unknown_keys(&input, &scene);
    if !unknown.is_empty() {
        return Err(format!("Unbekannte Felder (würden ignoriert):\n- {}", unknown.join("\n- ")));
    }
    Ok(scene)
}

const COMMON: &[&str] = &[
    "id", "name", "hidden", "position", "rotation", "scale", "material", "repeat", "animate", "type", "subdivide", "rounded", "deform", "billboard",
];

/// Felder in Objekten, die es nicht gibt (z. B. `"sise"` statt `"size"`).
/// serde ignoriert diese bei manchen Objekt-Typen, daher prüfen wir selbst.
pub fn unknown_keys(input: &Value, scene: &Scene) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(list) = input.get("objects").and_then(Value::as_array) {
        check_objects(list, &scene.objects, "objects", &mut out);
    }
    out
}

fn check_objects(inputs: &[Value], objs: &[Object], path: &str, out: &mut Vec<String>) {
    for (i, (inp, obj)) in inputs.iter().zip(objs).enumerate() {
        let p = format!("{path}[{i}] (\"{}\")", obj.id);
        check_object(inp, obj, &p, out);
    }
}

fn check_object(inp: &Value, obj: &Object, path: &str, out: &mut Vec<String>) {
    let Some(map) = inp.as_object() else { return };
    let ser = serde_json::to_value(&obj.kind).unwrap_or(Value::Null);
    let mut known: Vec<String> = COMMON.iter().map(|s| s.to_string()).collect();
    if let Some(m) = ser.as_object() {
        known.extend(m.keys().cloned());
    }
    // Optionale Felder, die beim Serialisieren weggelassen werden, wenn sie leer sind
    known.extend(["radius_end", "holes", "on_terrain", "material_override", "svg", "file", "mode", "resolution"].map(String::from));
    for key in map.keys() {
        if !known.iter().any(|k| k == key) {
            let hint = suggest(key, known.iter().map(String::as_str)).map(|s| format!(" → Meintest du „{s}“?")).unwrap_or_default();
            out.push(format!("{path}: unbekanntes Feld „{key}“ für type \"{}\"{hint}", map.get("type").and_then(Value::as_str).unwrap_or("?")));
        }
    }
    match &obj.kind {
        Kind::Group { children } => {
            if let Some(list) = map.get("children").and_then(Value::as_array) {
                check_objects(list, children, &format!("{path}.children"), out);
            }
        }
        Kind::Scatter(s) => {
            if let Some(shape) = map.get("shape").and_then(Value::as_object) {
                let ser = serde_json::to_value(&s.shape.kind).unwrap_or(Value::Null);
                for key in shape.keys() {
                    let ok = key == "type" || ser.get(key).is_some();
                    if !ok {
                        out.push(format!("{path}.shape: unbekanntes Feld „{key}“"));
                    }
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn suggestions() {
        assert_eq!(suggest("heigth", ["size", "height", "style"]), Some("height"));
        assert_eq!(suggest("spere", ["sphere", "box", "cone"]), Some("sphere"));
        assert_eq!(suggest("xyz", ["sphere", "box"]), None);
    }

    #[test]
    fn silent_typos_are_caught_with_path_and_hint() {
        let err = parse_scene(json!({"objects": [
            {"id": "ok", "type": "sphere"},
            {"id": "g", "type": "group", "children": [{"id": "kiste", "type": "box", "sise": [1, 2, 3]}]}
        ]}))
        .unwrap_err();
        assert!(err.contains("objects[1] (\"g\").children[0] (\"kiste\")"), "{err}");
        assert!(err.contains("„sise“") && err.contains("Meintest du „size“?"), "{err}");
    }

    #[test]
    fn serde_errors_get_path_and_suggestion() {
        let err = parse_scene(json!({"objects": [{"id": "a", "type": "spere"}]})).unwrap_err();
        assert!(err.contains("Meintest du „sphere“?"), "{err}");
        let err = parse_scene(json!({"objects": [{"id": "a", "type": "box", "material": {"colour": "#fff"}}]})).unwrap_err();
        assert!(err.contains("material") && err.contains("Meintest du „color“?"), "{err}");
        let err = parse_scene(json!({"objects": [{"id": "a", "type": "box", "material": {"color": "lila"}}]})).unwrap_err();
        assert!(err.contains("objects[0].material.color"), "Pfad fehlt: {err}");
    }

    #[test]
    fn valid_scenes_pass() {
        let demo = include_str!("../../../scenes/werkstatt.json");
        assert!(parse_scene(serde_json::from_str(demo).unwrap()).is_ok());
        let fx = include_str!("../../../scenes/effekte.json");
        assert!(parse_scene(serde_json::from_str(fx).unwrap()).is_ok(), "{:?}", parse_scene(serde_json::from_str(fx).unwrap()).err());
    }
}
