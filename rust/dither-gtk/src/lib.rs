//! Native GTK4 components backed by a two-pass GLSL display-list interpreter.
pub mod components;
pub mod renderer;
pub mod surface;
pub use components::*;
pub use dither_ir as ir;
pub use gtk;
pub use surface::DitherSurface;
