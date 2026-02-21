//! Entity normalisation pipeline.
//!
//! Two normalisers are provided:
//! - `HgncNormaliser`: maps gene symbols/aliases → canonical HGNC IDs
//! - `HgvsMutationNormaliser`: maps variant notations → HGVS p. / rsID

pub mod hgnc;
pub mod hgvs;

pub use hgnc::{HgncNormaliser, HgncRecord};
pub use hgvs::{HgvsMutationNormaliser, NormalisedMutation};
