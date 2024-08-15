//! Synchronization and interior mutability primitives
mod up;
pub use up::{UPIntrFreeCell, UPIntrRefMut};
