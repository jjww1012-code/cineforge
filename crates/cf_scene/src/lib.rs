//! # cf_scene
//!
//! Das Szenen-Dokument von CineForge: Datenmodell, Validierung und
//! prozedurale Geometrie. Bewusst ohne Abhängigkeit zur Render-Engine,
//! damit es von der App, dem Claude-Agenten, dem Server und Tests
//! gleichermaßen genutzt werden kann.

pub mod color;
pub mod model;
pub mod modeling;
pub mod noise;
pub mod ops;
pub mod project;
pub mod procedural;

pub use color::Rgb;
pub use model::*;

#[cfg(test)]
mod tests;
