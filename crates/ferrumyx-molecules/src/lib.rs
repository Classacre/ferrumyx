//! Ferrumyx Molecules - Structural analysis and molecule design pipeline.
//!
//! This crate handles Phase 5 of the Ferrumyx architecture:
//! 1. Fetching protein structures (PDB / AlphaFold)
//! 2. Detecting binding pockets (fpocket)
//! 3. Generating potential ligands
//! 4. Molecular docking (AutoDock Vina)
//! 5. ADMET prediction
//! 6. Scoring and ranking molecules

pub mod admet;
pub mod docking;
pub mod ligand;
pub mod pdb;
pub mod pipeline;
pub mod pocket;
pub mod scoring;

pub type Result<T> = anyhow::Result<T>;
