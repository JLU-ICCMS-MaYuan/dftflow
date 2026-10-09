//! QE 原生输入 deck 写出：pw / ph / q2r / matdyn（对齐 `qeflow/input_deck`）。

pub mod filenames;
pub mod matdyn;
pub mod ph;
pub mod pw;
pub mod q2r;
pub mod species;
pub mod values;

pub use filenames::Filenames;
pub use species::resolve_atomic_species_entries;
pub use values::to_qe_value;
