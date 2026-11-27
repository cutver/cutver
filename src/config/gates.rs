//! `[gates]`: the attestation channel a release reads (#326 slice 3B).
//!
//! A gate is a check somebody else ran — a CI job, a security scan, an end-to-end suite — and an
//! attestation is the verdict it recorded: *what* was verified (the gate), *for which commit*, *with
//! which conclusion*, and *when*. `cutver attest` writes one, `cutver publish --require-gate` reads
//! it before the first irreversible step, and `cutver plan --json` reports it as data.
//!
//! The store is one JSON object per gate, in the directory `[gates] dir` names. The object opens with
//! the `schemaVersion` every machine surface carries, and the record's own keys follow in the order
//! they are written:
//!
//! ```json
//! {"schemaVersion":1,"gate":"ci","commit":"<40 hex>","conclusion":"success","verified_at":"2026-11-19"}
//! ```
//!
//! A file written before the version key existed has none, and [`read`] still reads it: the absence
//! means the legacy shape, not a record to refuse.
//!
//! **Attestations are data, not trust (S12).** `cutver` records and reads them; it signs nothing,
//! verifies no signature and talks to no network, so an attestation is exactly as trustworthy as the
//! machine that wrote it. This tool can tell you a file *says* a gate passed; it cannot tell you the
//! gate passed. Treat the directory as part of the release inputs you protect, the way you protect
//! the token that pushes the tag.
//!
//! A file that does not parse, or that names another gate, is not an attestation: [`read`] treats it
//! as absent rather than failing, so a truncated or hand-edited file fails closed as "no
//! attestation for this commit" instead of crashing a release.
//!
//! A facade, per Contract Pillar V.4: the table lives in [`table`], the names and verdicts in
//! [`name`], and the recorded documents and the store in [`store`].

mod name;
mod store;
mod table;

#[cfg(test)]
mod tests;

pub use name::{Conclusion, GateName, GateNameError};
pub use store::{AttestError, Attestation, Recorded, declared_input, dir, path_of, read, record, relative};
pub use table::{DEFAULT_DIR, Gates};
