#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![recursion_limit = "512"]
//! CineForge Studio
//!
//! - lädt ein Szenen-Dokument (JSON) und baut daraus die 3D-Welt
//! - freie Flug-Kamera, Live-Reload, Rückgängig
//! - Claude-Verbindung (MCP) mit Werkzeugen zum Bauen und Prüfen
//! - Screenshot-Modus für automatische Bildkontrolle (`--screenshot`)

mod bridge;
mod capture;
mod claude_setup;
mod cli;
mod control;
mod crash;
mod environment;
mod film;
mod fx;
mod hud;
mod library;
mod media;
mod overlay;
mod project;
mod render;
mod source;
mod spawn;
mod studio_ui;
mod timeline;
mod tools;
mod video;

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};

use cli::Args;

fn main() -> AppExit {
    let args = match Args::parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("{msg}\n\n{}", cli::USAGE);
            return AppExit::error();
        }
    };
    if args.help {
        println!("{}", cli::USAGE);
        return AppExit::Success;
    }
    if args.mcp_bridge {
        bridge::run(args.port);
        return AppExit::Success;
    }

    crash::install();
    let source = match source::SceneSource::open(args.scene.clone()) {
        Ok(s) => s,
        // Einmaliger Screenshot: Fehler melden und beenden.
        Err(e) if args.screenshot.is_some() => {
            eprintln!("{e}");
            return AppExit::error();
        }
        // Normaler Start: nie stumm beenden, sondern mit der Demo-Szene öffnen und den Grund zeigen.
        Err(e) => {
            eprintln!("{e}");
            crash::note(&format!("Start: Szene nicht ladbar – öffne Demo. Grund: {e}"));
            source::SceneSource::fallback(&e)
        }
    };

    let (w, h) = args.size;
    let mut app = App::new();
    // Dateien im CineForge-Ordner (z. B. 3D-Modelle) als Asset-Quelle „cineforge://“.
    if let Ok(root) = project::root_dir() {
        let _ = std::fs::create_dir_all(&root);
        app.register_asset_source(
            "cineforge",
            bevy::asset::io::AssetSourceBuilder::platform_default(&root.to_string_lossy(), None),
        );
    }
    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "CineForge Studio".into(),
                resolution: WindowResolution::new(w, h),
                present_mode: if args.screenshot.is_some() {
                    PresentMode::AutoNoVsync
                } else {
                    PresentMode::AutoVsync
                },
                ..default()
            }),
            ..default()
        }),
    )
    .add_plugins((FreeCameraPlugin, FrameTimeDiagnosticsPlugin::default()))
    .insert_resource(ClearColor(Color::BLACK))
    .insert_resource(project::ActiveProject::for_scene(&source.path))
    .insert_resource(source)
    .add_plugins((
        environment::EnvironmentPlugin,
        spawn::SpawnPlugin,
        source::SourcePlugin,
        project::ProjectPlugin { remember: args.screenshot.is_none() },
        timeline::TimelinePlugin { interactive: args.screenshot.is_none() || args.hud },
        render::RenderPlugin,
        fx::FxPlugin,
        film::FilmPlugin,
        overlay::OverlayPlugin,
    ))
    // Auch im Hintergrund weiterrechnen, damit Claude arbeiten kann,
    // während das Fenster nicht im Vordergrund ist.
    .insert_resource(WinitSettings { focused_mode: UpdateMode::Continuous, unfocused_mode: UpdateMode::Continuous })
    .add_systems(Startup, setup_camera);

    if let Some(path) = args.screenshot.clone() {
        app.add_plugins(capture::AutoCapturePlugin { path, frames: args.frames, time: args.time });
        app.add_plugins(hud::HudPlugin);
        if args.hud {
            // Mit --hud zeigt der Screenshot die komplette Studio-Oberfläche.
            app.add_plugins(studio_ui::StudioUiPlugin);
            app.insert_resource(studio_ui::Selection { id: args.select.clone() });
        }
    } else {
        app.add_plugins((
            hud::HudPlugin,
            studio_ui::StudioUiPlugin,
            capture::ManualCapturePlugin,
            control::ControlPlugin { port: args.port },
            claude_setup::ClaudeSetupPlugin,
        ));
    }

    app.run()
}

#[derive(Component)]
pub struct MainCamera;

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        MainCamera,
        environment::SceneView,
        Camera3d::default(),
        Msaa::Sample4,
        FreeCamera {
            walk_speed: 20.0,
            run_speed: 80.0,
            friction: 20.0,
            ..default()
        },
    ));
}
