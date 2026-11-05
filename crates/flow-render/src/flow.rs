//! The CLI's own scene code, shared through the `flow` symlink to `src/flow`:
//! only the modules a scene needs, so every client draws exactly what the
//! terminal draws.
pub mod agents;
pub mod cells;
pub mod clouds;
pub mod crew;
pub mod js;
pub mod night;
pub mod palette;
pub mod pixel_scene;
pub mod pixels;
pub mod scene;
pub mod scenes;
pub mod sky;
pub mod waiting;
#[cfg(test)]
pub mod reference;
