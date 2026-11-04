#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
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
mod environment;
mod fx;
mod hud;
mod library;
mod media;
mod project;
mod render;
mod source;
mod spawn;
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

    let source = match source::SceneSource::open(args.scene.clone()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return AppExit::error();
        }
    };

    let (w, h) = args.size;
    let mut app = App::new();
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
        project::ProjectPlugin,
        timeline::TimelinePlugin { interactive: args.screenshot.is_none() || args.hud },
        render::RenderPlugin,
        fx::FxPlugin,
    ))
    // Auch im Hintergrund weiterrechnen, damit Claude arbeiten kann,
    // während das Fenster nicht im Vordergrund ist.
    .insert_resource(WinitSettings { focused_mode: UpdateMode::Continuous, unfocused_mode: UpdateMode::Continuous })
    .add_systems(Startup, setup_camera);

    if let Some(path) = args.screenshot.clone() {
        app.add_plugins(capture::AutoCapturePlugin { path, frames: args.frames, time: args.time });
        if args.hud {
            app.add_plugins(hud::HudPlugin);
        }
    } else {
        app.add_plugins((
            hud::HudPlugin,
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
