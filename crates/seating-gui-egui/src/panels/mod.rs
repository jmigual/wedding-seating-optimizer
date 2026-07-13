//! UI panels. `editors` and `canvas` are owned exclusively by their own
//! files (plus tests) per the parallel-stage discipline in the spec: later
//! work on either must not touch `app.rs`, `state.rs`, `main.rs`, or the
//! sibling panel file.

pub(crate) mod canvas;
pub(crate) mod editors;
pub(crate) mod status_bar;
pub(crate) mod top_bar;

pub(crate) use canvas::CanvasState;
pub(crate) use editors::EditorsState;
