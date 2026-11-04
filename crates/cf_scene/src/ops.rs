//! Bearbeitungs-Operationen auf dem Szenen-Dokument.
//!
//! Alle Operationen arbeiten auf einer Kopie und liefern entweder die neue,
//! **gültige** Szene oder einen verständlichen Fehler. Die alte Szene bleibt
//! dabei unangetastet. So kann ein fehlerhafter Befehl (von Claude oder aus
//! der UI) nie eine kaputte Szene hinterlassen.

use serde_json::{Map, Value};

use crate::model::{Kind, Object, Scene};

pub type OpResult = Result<Scene, String>;

/// JSON-Merge-Patch nach RFC 7396: Objekte werden zusammengeführt,
/// `null` löscht ein Feld, alles andere ersetzt.
pub fn merge_patch(target: &mut Value, patch: &Value) {
    match patch {
        Value::Object(p) => {
            if !target.is_object() {
                *target = Value::Object(Map::new());
            }
            let t = target.as_object_mut().expect("eben gesetzt");
            for (k, v) in p {
                if v.is_null() {
                    t.remove(k);
                } else {
                    merge_patch(t.entry(k.clone()).or_insert(Value::Null), v);
                }
            }
        }
        _ => *target = patch.clone(),
    }
}

fn to_value(scene: &Scene) -> Value {
    serde_json::to_value(scene).expect("Szene ist immer serialisierbar")
}

fn from_value(v: Value) -> OpResult {
    let scene: Scene = serde_json::from_value(v).map_err(|e| format!("Ungültige Angaben: {e}"))?;
    let problems = scene.validate();
    if problems.is_empty() {
        Ok(scene)
    } else {
        Err(format!("Szene wäre ungültig:\n- {}", problems.join("\n- ")))
    }
}

/// Sucht ein Objekt (auch in Gruppen) im JSON-Baum und gibt es veränderbar zurück.
fn find_value_mut<'a>(objects: &'a mut Value, id: &str) -> Option<&'a mut Value> {
    for o in objects.as_array_mut()? {
        if o.get("id").and_then(Value::as_str) == Some(id) {
            return Some(o);
        }
        if let Some(children) = o.get_mut("children") {
            if let Some(found) = find_value_mut(children, id) {
                return Some(found);
            }
        }
    }
    None
}

/// Ändert einzelne Eigenschaften eines Objekts, z. B.
/// `{"material": {"color": "#ff0000"}, "position": [0, 5, 0]}`.
pub fn update_object(scene: &Scene, id: &str, changes: &Value) -> OpResult {
    if !changes.is_object() {
        return Err("changes muss ein JSON-Objekt sein".into());
    }
    let mut v = to_value(scene);
    let obj = find_value_mut(&mut v["objects"], id).ok_or_else(|| unknown_id(scene, id))?;
    // Typwechsel (z. B. box → sphere): typspezifische Altfelder verwerfen,
    // sonst bleiben z. B. "size"-Felder einer Box an einer Kugel hängen.
    if let Some(new_type) = changes.get("type").and_then(Value::as_str) {
        if obj.get("type").and_then(Value::as_str) != Some(new_type) {
            let keep = ["id", "name", "hidden", "position", "rotation", "scale", "material"];
            obj.as_object_mut()
                .expect("Objekte sind JSON-Objekte")
                .retain(|k, _| keep.contains(&k.as_str()));
        }
    }
    merge_patch(obj, changes);
    from_value(v)
}

/// Fügt Objekte hinzu. Existiert eine id bereits, wird das Objekt ersetzt.
/// Mit `parent` landen die Objekte als Kinder in dieser Gruppe.
pub fn upsert_objects(scene: &Scene, objects: &[Value], parent: Option<&str>) -> OpResult {
    let mut v = to_value(scene);
    for new in objects {
        let id = new
            .get("id")
            .and_then(Value::as_str)
            .ok_or("Jedes Objekt braucht eine \"id\"")?
            .to_string();
        if let Some(existing) = find_value_mut(&mut v["objects"], &id) {
            *existing = new.clone();
            continue;
        }
        let list = match parent {
            None => &mut v["objects"],
            Some(pid) => {
                let p = find_value_mut(&mut v["objects"], pid).ok_or_else(|| unknown_id(scene, pid))?;
                if p.get("type").and_then(Value::as_str) != Some("group") {
                    return Err(format!("\"{pid}\" ist keine Gruppe (type \"group\")"));
                }
                p.as_object_mut()
                    .expect("Objekte sind JSON-Objekte")
                    .entry("children")
                    .or_insert_with(|| Value::Array(vec![]))
            }
        };
        list.as_array_mut().ok_or("children muss eine Liste sein")?.push(new.clone());
    }
    from_value(v)
}

pub fn remove_objects(scene: &Scene, ids: &[String]) -> OpResult {
    fn rec(list: &mut Vec<Object>, ids: &[String], removed: &mut Vec<String>) {
        list.retain(|o| {
            let hit = ids.contains(&o.id);
            if hit {
                removed.push(o.id.clone());
            }
            !hit
        });
        for o in list.iter_mut() {
            if let Kind::Group { children } = &mut o.kind {
                rec(children, ids, removed);
            }
        }
    }
    let mut s = scene.clone();
    let mut removed = Vec::new();
    rec(&mut s.objects, ids, &mut removed);
    let missing: Vec<_> = ids.iter().filter(|i| !removed.contains(i)).collect();
    if !missing.is_empty() {
        return Err(format!("Unbekannte id(s): {missing:?}. Vorhanden: {}", id_list(scene)));
    }
    // Verweise prüfen (z. B. scatter.on_terrain auf ein gelöschtes Terrain).
    let problems = s.validate();
    if !problems.is_empty() {
        return Err(format!("Nach dem Löschen wäre die Szene ungültig:\n- {}", problems.join("\n- ")));
    }
    Ok(s)
}

/// Ändert Szenen-Einstellungen (name, description, environment, camera).
/// Objekte werden über die Objekt-Operationen geändert.
pub fn update_settings(scene: &Scene, changes: &Value) -> OpResult {
    let Some(map) = changes.as_object() else {
        return Err("changes muss ein JSON-Objekt sein".into());
    };
    if map.contains_key("objects") {
        return Err("Objekte bitte mit add_objects / update_object / remove_objects ändern".into());
    }
    let mut v = to_value(scene);
    merge_patch(&mut v, changes);
    from_value(v)
}

/// Kurze Übersicht als Baum, z. B. für Claude oder die Objektliste.
pub fn outline(scene: &Scene) -> String {
    fn kind_label(k: &Kind) -> String {
        match k {
            Kind::Box { size } => format!("box {size:?}"),
            Kind::Sphere { radius } => format!("sphere r={radius}"),
            Kind::Cylinder { radius, height } => format!("cylinder r={radius} h={height}"),
            Kind::Cone { radius, height } => format!("cone r={radius} h={height}"),
            Kind::Capsule { radius, length } => format!("capsule r={radius} l={length}"),
            Kind::Torus { radius, thickness } => format!("torus r={radius} t={thickness}"),
            Kind::Plane { size } => format!("plane {size:?}"),
            Kind::Pyramid { size } => format!("pyramid {size:?}"),
            Kind::Terrain(t) => format!("terrain {:?} {:?} h={}", t.style, t.size, t.height),
            Kind::Scatter(s) => format!("scatter ×{}", s.count),
            Kind::Group { children } => format!("group ({} Kinder)", children.len()),
            Kind::PointLight(l) => format!("point_light {} lm", l.intensity),
            Kind::RoundedBox { size, radius, .. } => format!("rounded_box {size:?} r={radius}"),
            Kind::Extrude(e) => format!("extrude {} Punkte h={}", e.profile.len(), e.height),
            Kind::Lathe(l) => format!("lathe {} Punkte", l.profile.len()),
            Kind::Tube(t) => format!("tube {} Punkte r={}", t.path.len(), t.radius),
            Kind::Text(t) => format!("text \"{}\"", t.text.chars().take(30).collect::<String>()),
            Kind::Instance { asset, .. } => format!("instance \"{asset}\""),
            Kind::Particles(p) => format!("particles {:?}", p.preset).to_lowercase(),
        }
    }
    fn rec(out: &mut String, objs: &[Object], depth: usize) {
        for o in objs {
            let p = o.position;
            let rep = o.repeat.as_ref().map(|r| format!(" ×{}", r.count)).unwrap_or_default();
            out.push_str(&format!(
                "{}- {} [{}{rep}] @ [{:.1}, {:.1}, {:.1}]{}{}\n",
                "  ".repeat(depth),
                o.id,
                kind_label(&o.kind),
                p[0],
                p[1],
                p[2],
                o.name.as_ref().map(|n| format!(" „{n}“")).unwrap_or_default(),
                if o.hidden { " (versteckt)" } else { "" }
            ));
            if let Kind::Group { children } = &o.kind {
                rec(out, children, depth + 1);
            }
        }
    }
    let mut out = format!(
        "Szene: {}\n{}\nKamera: {:?} → {:?}, fov {}°\nSonne: Azimut {}°, Höhe {}°\nObjekte:\n",
        if scene.name.is_empty() { "(ohne Namen)" } else { &scene.name },
        scene.description,
        scene.camera.position,
        scene.camera.look_at,
        scene.camera.fov,
        scene.environment.sun.azimuth,
        scene.environment.sun.elevation,
    );
    if scene.objects.is_empty() {
        out.push_str("(keine)\n");
    }
    rec(&mut out, &scene.objects, 0);
    out
}

fn id_list(scene: &Scene) -> String {
    let mut ids = Vec::new();
    scene.walk(&mut |o| ids.push(o.id.as_str()));
    if ids.len() > 40 {
        format!("{} … (insgesamt {})", ids[..40].join(", "), ids.len())
    } else {
        ids.join(", ")
    }
}

fn unknown_id(scene: &Scene, id: &str) -> String {
    format!("Kein Objekt mit id \"{id}\". Vorhanden: {}", id_list(scene))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base() -> Scene {
        Scene::from_json(
            r#"{"objects":[
                {"id":"ground","type":"terrain","resolution":8},
                {"id":"g","type":"group","children":[{"id":"cube","type":"box"}]},
                {"id":"rocks","type":"scatter","shape":{"type":"sphere"},"count":3,"on_terrain":"ground"}
            ]}"#,
        )
        .unwrap()
    }

    #[test]
    fn merge_patch_follows_rfc7396() {
        let mut v = json!({"a": 1, "b": {"c": 2, "d": 3}});
        merge_patch(&mut v, &json!({"a": null, "b": {"c": 9}, "e": [1]}));
        assert_eq!(v, json!({"b": {"c": 9, "d": 3}, "e": [1]}));
    }

    #[test]
    fn update_nested_object_partially() {
        let s = update_object(&base(), "cube", &json!({"material": {"color": "#ff0000"}, "position": [1, 2, 3]})).unwrap();
        let c = s.find("cube").unwrap();
        assert_eq!(c.position, [1.0, 2.0, 3.0]);
        assert_eq!(c.material.color.to_hex(), "#ff0000");
        assert_eq!(c.material.roughness, 0.6, "andere Materialwerte bleiben");
    }

    #[test]
    fn changing_type_drops_old_fields() {
        let s = update_object(&base(), "cube", &json!({"type": "sphere", "radius": 2})).unwrap();
        assert!(matches!(s.find("cube").unwrap().kind, Kind::Sphere { radius } if radius == 2.0));
    }

    #[test]
    fn invalid_update_is_rejected_and_explained() {
        let err = update_object(&base(), "cube", &json!({"material": {"color": "lila"}})).unwrap_err();
        assert!(err.contains("lila"), "{err}");
        let err = update_object(&base(), "nope", &json!({})).unwrap_err();
        assert!(err.contains("cube"), "nennt vorhandene ids: {err}");
    }

    #[test]
    fn upsert_adds_replaces_and_nests() {
        let s = upsert_objects(
            &base(),
            &[json!({"id":"ball","type":"sphere"}), json!({"id":"cube","type":"cone"})],
            None,
        )
        .unwrap();
        assert!(s.find("ball").is_some());
        assert!(matches!(s.find("cube").unwrap().kind, Kind::Cone { .. }), "ersetzt in der Gruppe");
        let s = upsert_objects(&s, &[json!({"id":"kid","type":"box"})], Some("g")).unwrap();
        let Kind::Group { children } = &s.find("g").unwrap().kind else { panic!() };
        assert_eq!(children.len(), 2);
        assert!(upsert_objects(&s, &[json!({"id":"x","type":"box"})], Some("cube")).is_err());
        assert!(upsert_objects(&s, &[json!({"type":"box"})], None).is_err());
    }

    #[test]
    fn remove_checks_ids_and_references() {
        let s = remove_objects(&base(), &["cube".into()]).unwrap();
        assert!(s.find("cube").is_none());
        assert!(remove_objects(&base(), &["nope".into()]).is_err());
        let err = remove_objects(&base(), &["ground".into()]).unwrap_err();
        assert!(err.contains("on_terrain"), "{err}");
    }

    #[test]
    fn settings_update_keeps_objects() {
        let s = update_settings(&base(), &json!({"name": "Neu", "environment": {"sun": {"elevation": 60}}})).unwrap();
        assert_eq!(s.name, "Neu");
        assert_eq!(s.environment.sun.elevation, 60.0);
        assert_eq!(s.environment.sun.azimuth, 135.0);
        assert_eq!(s.objects.len(), 3);
        assert!(update_settings(&base(), &json!({"objects": []})).is_err());
    }

    #[test]
    fn outline_lists_tree() {
        let o = outline(&base());
        assert!(o.contains("- g [group (1 Kinder)]"), "{o}");
        assert!(o.contains("  - cube [box"), "{o}");
    }
}
