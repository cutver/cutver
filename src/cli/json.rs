//! The machine contract every JSON surface shares.
//!
//! One rule holds for all of them: the first key of the object cutver emits is `schemaVersion`, and
//! it is [`SCHEMA_VERSION`]. A version change means an existing key changed meaning or disappeared.
//! Adding a key is *not* a new version, because a consumer that ignores unknown keys keeps reading
//! the same values (Pillar IV.2: change is additive).

use serde::Serialize;

/// The version of the JSON schema every machine surface emits.
///
/// `1` is the first published schema. It covers the release result (`bump --json`, `bump --report`),
/// the release context (`changelog latest|show --json`), the plugin surfaces (`plugin list|info|search
/// --json`) and the diagnosis (`doctor --json`).
pub const SCHEMA_VERSION: u32 = 1;

/// Wraps a payload struct in the version envelope, so `schemaVersion` is the first key and the
/// payload's own keys follow in declaration order.
///
/// The payload must serialize as an object; a map, a struct or a `serde_json::Map` all do. A payload
/// that serializes as an array or a scalar cannot be flattened and fails at serialization, which is
/// the intended boundary: a bare array has no room for a version, so it gets an object wrapper.
#[derive(Debug, Serialize)]
pub struct Versioned<'a, T: Serialize> {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(flatten)]
    payload: &'a T,
}

impl<'a, T: Serialize> Versioned<'a, T> {
    /// Wraps `payload` with the current [`SCHEMA_VERSION`].
    pub fn new(payload: &'a T) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Payload {
        released: bool,
        tag: &'static str,
    }

    #[test]
    fn the_schema_version_is_the_first_key_and_the_payload_keeps_its_order() {
        let wrapped = Versioned::new(&Payload {
            released: true,
            tag: "v1.0.0",
        });

        let json = serde_json::to_string(&wrapped).expect("the envelope serializes");

        assert_eq!(json, r#"{"schemaVersion":1,"released":true,"tag":"v1.0.0"}"#);
    }

    #[test]
    fn a_map_payload_is_flattened_next_to_the_version() {
        let payload = serde_json::json!({ "plugins": [] });

        let json = serde_json::to_string(&Versioned::new(&payload)).expect("the envelope serializes");

        assert_eq!(json, r#"{"schemaVersion":1,"plugins":[]}"#);
    }

    #[test]
    fn an_array_payload_fails_closed_instead_of_losing_the_version() {
        let payload = vec!["a"];

        let error = serde_json::to_string(&Versioned::new(&payload));

        assert!(
            error.is_err(),
            "a bare array has no room for a version, so it must be wrapped in an object"
        );
    }
}
