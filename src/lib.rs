//! Library surface for reporadar. The binary (`main.rs`) is a thin wrapper;
//! exposing the modules here lets integration tests drive the real
//! scan -> worker -> app -> render pipeline end-to-end.

pub mod app;
pub mod event;
pub mod external;
pub mod git;
pub mod scan;
pub mod tui;
pub mod ui;
pub mod worker;
