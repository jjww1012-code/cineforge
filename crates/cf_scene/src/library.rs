//! Objekt-Bibliothek: einmal gebaute Objekte (Raumschiff, Baum, Haus …) als
//! eigene Datei speichern und in beliebig vielen Szenen per `instance`
//! wiederverwenden. So sieht ein Objekt in jeder Szene gleich aus.

use serde::{Deserialize, Serialize};

use crate::model::{Kind, Object, Scene};

/// Inhalt einer Bibliotheks-Datei (`<name>.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AssetFile {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Das Objekt (meist eine Gruppe). Ursprung = Bezugspunkt beim Platzieren,
    /// sinnvollerweise unten in der Mitte.
    pub object: Object,
}

impl AssetFile {
    pub fn from_json(text: &str) -> Result<Self, String> {
        let a: AssetFile = serde_json::from_str(text).map_err(|e| format!("Bibliotheks-Datei ungültig: {e}"))?;
        a.validate()?;
        Ok(a)
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialisierbar")
    }

    /// Prüft das Objekt mit denselben Regeln wie eine Szene.
    pub fn validate(&self) -> Result<(), String> {
        let wrapper = Scene { objects: vec![self.object.clone()], ..Scene::from_json("{}").expect("leer") };
        let problems = wrapper.validate();
        if problems.is_empty() {
            Ok(())
        } else {
            Err(format!("Objekt ist ungültig:\n- {}", problems.join("\n- ")))
        }
    }
}

/// Alle Bibliotheks-Namen, auf die ein Objekt(baum) verweist.
pub fn referenced_assets(objects: &[Object], out: &mut Vec<String>) {
    for o in objects {
        match &o.kind {
            Kind::Instance { asset, .. } => {
                if !out.contains(asset) {
                    out.push(asset.clone());
                }
            }
            Kind::Group { children } => referenced_assets(children, out),
            _ => {}
        }
    }
}

/// Grobe Abmessungen eines Objekts (ohne Rotation), für die Bibliotheks-Übersicht.
pub fn approx_extent(o: &Object) -> Option<([f32; 3], [f32; 3])> {
    fn rec(o: &Object, offset: [f32; 3], scale: [f32; 3], acc: &mut Option<([f32; 3], [f32; 3])>) {
        let s = o.scale.to_array();
        let sc = [scale[0] * s[0], scale[1] * s[1], scale[2] * s[2]];
        let pos = [
            offset[0] + o.position[0] * scale[0],
            offset[1] + o.position[1] * scale[1],
            offset[2] + o.position[2] * scale[2],
        ];
        let half: Option<([f32; 3], [f32; 3])> = match &o.kind {
            Kind::Box { size } | Kind::RoundedBox { size, .. } => {
                Some(([-size[0] / 2.0, -size[1] / 2.0, -size[2] / 2.0], [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0]))
            }
            Kind::Pyramid { size } => Some(([-size[0] / 2.0, 0.0, -size[2] / 2.0], [size[0] / 2.0, size[1], size[2] / 2.0])),
            Kind::Sphere { radius } => Some(([-radius; 3], [*radius; 3])),
            Kind::Cylinder { radius, height } | Kind::Cone { radius, height } => {
                Some(([-radius, -height / 2.0, -radius], [*radius, height / 2.0, *radius]))
            }
            Kind::Extrude(e) => {
                let (mut lo, mut hi) = ([f32::MAX, 0.0, f32::MAX], [f32::MIN, e.height, f32::MIN]);
                for p in &e.profile {
                    lo[0] = lo[0].min(p[0]);
                    hi[0] = hi[0].max(p[0]);
                    lo[2] = lo[2].min(p[1]);
                    hi[2] = hi[2].max(p[1]);
                }
                Some((lo, hi))
            }
            Kind::Lathe(l) => {
                let r = l.profile.iter().map(|p| p[0]).fold(0.0, f32::max);
                let (y0, y1) = l.profile.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[1]), b.max(p[1])));
                Some(([-r, y0, -r], [r, y1, r]))
            }
            Kind::Tube(t) => {
                let r = t.radius.max(t.radius_end.unwrap_or(0.0));
                let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                for p in &t.path {
                    for i in 0..3 {
                        lo[i] = lo[i].min(p[i] - r);
                        hi[i] = hi[i].max(p[i] + r);
                    }
                }
                Some((lo, hi))
            }
            Kind::Capsule { radius, length } => {
                Some(([-radius, -length / 2.0 - radius, -radius], [*radius, length / 2.0 + radius, *radius]))
            }
            Kind::Torus { radius, thickness } => {
                let o = radius + thickness;
                Some(([-o, -thickness, -o], [o, *thickness, o]))
            }
            Kind::Plane { size } => Some(([-size[0] / 2.0, 0.0, -size[1] / 2.0], [size[0] / 2.0, 0.0, size[1] / 2.0])),
            Kind::Text(t) => {
                let lines = t.text.lines().count().max(1) as f32;
                let w = t.text.lines().map(|l| l.chars().count()).max().unwrap_or(0) as f32 * t.size * 0.6;
                Some(([-w / 2.0, -(lines - 1.0) * t.size * 1.2, -t.depth / 2.0], [w / 2.0, t.size * 0.75, t.depth / 2.0]))
            }
            Kind::Group { children } => {
                for c in children {
                    rec(c, pos, sc, acc);
                }
                None
            }
            _ => None,
        };
        if let Some((lo, hi)) = half {
            let lo = [pos[0] + lo[0] * sc[0], pos[1] + lo[1] * sc[1], pos[2] + lo[2] * sc[2]];
            let hi = [pos[0] + hi[0] * sc[0], pos[1] + hi[1] * sc[1], pos[2] + hi[2] * sc[2]];
            let e = acc.get_or_insert((lo, hi));
            for i in 0..3 {
                e.0[i] = e.0[i].min(lo[i]).min(hi[i]);
                e.1[i] = e.1[i].max(hi[i]).max(lo[i]);
            }
        }
    }
    let mut acc = None;
    rec(o, [0.0; 3], [1.0; 3], &mut acc);
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_roundtrip_validation_and_refs() {
        let a = AssetFile::from_json(
            r#"{"name":"baum","description":"Laubbaum","object":{"id":"baum","type":"group","children":[
                {"id":"stamm","type":"cylinder","radius":0.2,"height":3,"position":[0,1.5,0]},
                {"id":"ast","type":"tube","path":[[0,2,0],[1.5,6,0]],"radius":0.1},
                {"id":"krone","type":"sphere","radius":1.5,"position":[0,4,0]}]}}"#,
        )
        .unwrap();
        assert_eq!(AssetFile::from_json(&a.to_json_pretty()).unwrap(), a);
        let (lo, hi) = approx_extent(&a.object).unwrap();
        assert!((lo[1] - 0.0).abs() < 1e-4 && (hi[1] - 6.1).abs() < 1e-4, "Röhre zählt mit: {lo:?} {hi:?}");
        assert!(AssetFile::from_json(r#"{"name":"x","object":{"id":"x","type":"sphere","radius":-1}}"#).is_err());

        let s = Scene::from_json(
            r#"{"objects":[{"id":"a","type":"instance","asset":"baum"},
               {"id":"g","type":"group","children":[{"id":"b","type":"instance","asset":"haus"},{"id":"c","type":"instance","asset":"baum"}]}]}"#,
        )
        .unwrap();
        let mut refs = vec![];
        referenced_assets(&s.objects, &mut refs);
        assert_eq!(refs, vec!["baum".to_string(), "haus".to_string()]);
    }
}
