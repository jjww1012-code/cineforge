use crate::procedural::*;
use crate::*;

const DEMO: &str = include_str!("../../../scenes/wueste.json");

#[test]
fn demo_scene_is_valid_and_roundtrips() {
    let scene = Scene::from_json(DEMO).expect("Demo-Szene muss gültig sein");
    assert!(scene.find("pyramid_great").is_some());
    assert!(scene.find("obelisk_tip").is_some(), "Gruppen-Kinder werden gefunden");
    let again = Scene::from_json(&scene.to_json_pretty()).unwrap();
    assert_eq!(scene, again);
}

#[test]
fn minimal_scene_uses_defaults() {
    let s = Scene::from_json(r#"{"objects":[{"id":"a","type":"box"}]}"#).unwrap();
    assert_eq!(s.objects[0].scale.to_array(), [1.0; 3]);
    assert!(matches!(s.environment.sky, Sky::Atmosphere));
}

#[test]
fn duplicate_ids_and_bad_references_are_reported() {
    let err = Scene::from_json(
        r#"{"objects":[
            {"id":"a","type":"box"},
            {"id":"a","type":"sphere"},
            {"id":"s","type":"scatter","shape":{"type":"box"},"count":3,"on_terrain":"nope"}
        ]}"#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("doppelt"), "{msg}");
    assert!(msg.contains("nope"), "{msg}");
}

#[test]
fn unknown_kind_fields_are_rejected() {
    let err = Scene::from_json(
        r#"{"objects":[{"id":"t","type":"terrain","heigth":5}]}"#,
    );
    assert!(err.is_err(), "Tippfehler in Feldnamen sollen auffallen");
}

#[test]
fn colors_parse_hex_and_arrays() {
    assert_eq!(Rgb::parse_hex("#fff"), Some(Rgb::WHITE));
    assert_eq!(Rgb::parse_hex("#ff0000"), Some(Rgb::new(1.0, 0.0, 0.0)));
    let m: Material = serde_json::from_str(r#"{"color":[0.5,0.5,0.5]}"#).unwrap();
    assert_eq!(m.color, Rgb::new(0.5, 0.5, 0.5));
    assert!(serde_json::from_str::<Material>(r#"{"color":"rot"}"#).is_err());
}

#[test]
fn terrain_is_deterministic_and_within_bounds() {
    let spec = TerrainSpec {
        size: [100.0, 100.0],
        resolution: 64,
        height: 10.0,
        style: TerrainStyle::Dunes,
        feature_size: 30.0,
        seed: 42,
        flat_center_radius: 10.0,
    };
    let a = terrain_mesh(&spec);
    let b = terrain_mesh(&spec);
    assert_eq!(a.positions, b.positions);
    assert_eq!(a.positions.len(), 64 * 64);
    assert_eq!(a.triangle_count(), 63 * 63 * 2);
    for p in &a.positions {
        assert!(p[1] >= 0.0 && p[1] <= 10.0 + 1e-3, "Höhe außerhalb: {}", p[1]);
    }
    // Mitte bleibt flach
    let hf = Heightfield::new(&spec);
    assert!(hf.height(0.0, 0.0).abs() < 1e-4);
    // Normalen zeigen nach oben
    assert!(a.normals.iter().all(|n| n[1] > 0.0));
}

#[test]
fn every_terrain_style_produces_variation() {
    for style in [TerrainStyle::Hills, TerrainStyle::Dunes, TerrainStyle::Mountains] {
        let spec = TerrainSpec {
            size: [200.0, 200.0],
            resolution: 32,
            height: 10.0,
            style,
            feature_size: 40.0,
            seed: 1,
            flat_center_radius: 0.0,
        };
        let m = terrain_mesh(&spec);
        let (lo, hi) = m
            .positions
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p[1]), hi.max(p[1])));
        assert!(hi - lo > 2.0, "{style:?} ist zu flach: {lo}..{hi}");
    }
}

#[test]
fn pyramid_faces_point_outwards() {
    let m = pyramid_mesh([2.0, 1.0, 2.0]);
    for tri in m.indices.chunks(3) {
        let n = m.normals[tri[0] as usize];
        let c = tri.iter().fold([0.0; 3], |acc, &i| {
            let p = m.positions[i as usize];
            [acc[0] + p[0] / 3.0, acc[1] + p[1] / 3.0, acc[2] + p[2] / 3.0]
        });
        // Vektor von der Pyramidenmitte zur Flächenmitte zeigt in Richtung der Normalen.
        let to_face = [c[0], c[1] - 0.25, c[2]];
        let dot = to_face[0] * n[0] + to_face[1] * n[1] + to_face[2] * n[2];
        assert!(dot > 0.0, "Fläche zeigt nach innen");
    }
}

#[test]
fn scatter_respects_count_clear_radius_and_ground() {
    let scene = Scene::from_json(DEMO).unwrap();
    let rocks = scene.find("rocks").unwrap();
    let Kind::Scatter(spec) = &rocks.kind else { panic!() };
    let (t, off, sc) = find_terrain(&scene.objects, "ground").unwrap();
    let hf = Heightfield::new(t);
    let ground = Ground { heightfield: &hf, offset: off, scale: sc };
    let inst = scatter_instances(spec, rocks.position, Some(&ground));
    assert_eq!(inst.len(), spec.count as usize);
    for i in &inst {
        let [x, y, z] = i.position;
        assert!((x * x + z * z).sqrt() >= spec.keep_clear_radius);
        let g = ground.world_height(x, z);
        // Kugel mit Radius 1·scale, 35 % eingesunken ⇒ Mittelpunkt knapp über dem Boden
        assert!((y - (g + (1.0 - 0.35 * 2.0) * i.scale)).abs() < 1e-3);
    }
}

#[test]
fn all_example_scenes_are_valid() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scenes");
    let mut files = vec![];
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "json") {
                files.push(p);
            }
        }
    }
    assert!(files.len() >= 8, "Beispielszenen gefunden: {}", files.len());
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        if let Err(e) = Scene::from_json(&text) {
            panic!("{}: {e}", f.display());
        }
    }
}
