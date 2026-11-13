//! Rust side of the Rust/Swift parity suite. The same vectors are checked by
//! `Tests/SwiftTests/ParityTests.swift`. Expected outcomes are written by hand in the vector file.

use homei_core::compatibility::{self, CompatibilityDb, CompatibilityStatus, Declaration};
use homei_core::data::Fixture;
use homei_core::device_profile::{looks_like_identifier, DeviceProfile, Provenance};
use homei_core::diagnostics::contains_identifier_like_token;
use homei_core::evidence::{Confidence, Evidence, EvidenceClass, EvidenceStore};
use homei_core::research::{Finding, FindingStatus};
use homei_core::safety::{self, Operation, SafetyDecision};
use serde_json::Value;
use std::path::Path;

fn vectors() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("golden/parity-vectors.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn list<'a>(v: &'a Value, key: &str) -> &'a Vec<Value> {
    v[key].as_array().unwrap_or_else(|| panic!("missing {key}"))
}

fn s(v: &Value) -> &str {
    v.as_str().unwrap()
}

fn status_name(status: CompatibilityStatus) -> String {
    serde_json::to_value(status)
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn vector_file_is_populated() {
    let v = vectors();
    for key in [
        "profile_validation",
        "identifier_like",
        "identifier_token",
        "decisions",
        "safety",
        "evidence_class",
        "confidence",
        "finding_status",
        "compatibility_status",
        "provenance",
        "records",
    ] {
        assert!(!list(&v, key).is_empty(), "{key} is empty");
    }
}

#[test]
fn device_profile_validation_matches_vectors() {
    for case in list(&vectors(), "profile_validation") {
        let outcome = match serde_json::from_str::<DeviceProfile>(s(&case["json"])) {
            Err(_) => "rejected_decode".to_string(),
            Ok(p) => match p.validate() {
                Ok(()) => "valid".to_string(),
                Err(e) => format!("rejected_validate:{}", e.field),
            },
        };
        assert_eq!(outcome, s(&case["expect"]), "{}", s(&case["name"]));
    }
}

#[test]
fn identifier_heuristics_match_vectors() {
    let v = vectors();
    for case in list(&v, "identifier_like") {
        assert_eq!(
            looks_like_identifier(s(&case["value"])),
            case["expect"].as_bool().unwrap(),
            "{:?}",
            case["value"]
        );
    }
    for case in list(&v, "identifier_token") {
        assert_eq!(
            contains_identifier_like_token(s(&case["text"])),
            case["expect"].as_bool().unwrap(),
            "{:?}",
            case["text"]
        );
    }
}

#[test]
fn compatibility_decisions_match_vectors() {
    for case in list(&vectors(), "decisions") {
        let name = s(&case["name"]);
        let profile: DeviceProfile = serde_json::from_value(case["profile"].clone()).unwrap();
        let declarations: Vec<Declaration> =
            serde_json::from_value(case["declarations"].clone()).unwrap();
        let evidence: Vec<Evidence> = serde_json::from_value(case["evidence"].clone()).unwrap();
        let d = compatibility::evaluate(
            &profile,
            &CompatibilityDb { declarations },
            &EvidenceStore::new(evidence),
        );
        let expect = &case["expect"];
        assert_eq!(
            status_name(d.status),
            s(&expect["status"]),
            "{name}: status"
        );
        assert_eq!(
            d.matched_declaration.as_deref(),
            expect["matched"].as_str(),
            "{name}: matched record"
        );
        assert_eq!(
            d.downgraded,
            expect["downgraded"].as_bool().unwrap(),
            "{name}: downgraded"
        );
        // A positive state must never appear without an exact-scope record behind it.
        if d.status.is_positive() {
            assert!(d.matched_declaration.is_some() && !d.downgraded, "{name}");
        }
    }
}

#[test]
fn safety_decisions_match_vectors() {
    for case in list(&vectors(), "safety") {
        let op = s(&case["operation"]);
        let decision = safety::authorize(&Operation::parse(op));
        let allowed = case["allowed"].as_bool().unwrap();
        assert_eq!(decision.is_allowed(), allowed, "{op:?}");
        match decision {
            SafetyDecision::Allowed => assert!(case["reason"].is_null(), "{op:?}"),
            SafetyDecision::Denied { reason } => {
                assert_eq!(reason, s(&case["reason"]), "{op:?}: reason text")
            }
        }
    }
}

fn accepts<T: serde::de::DeserializeOwned>(value: &str) -> bool {
    serde_json::from_str::<T>(&format!("\"{value}\"")).is_ok()
}

#[test]
fn closed_vocabularies_match_vectors() {
    let v = vectors();
    let check = |key: &str, f: &dyn Fn(&str) -> bool| {
        for case in list(&v, key) {
            let value = s(&case["value"]);
            assert_eq!(
                f(value),
                case["valid"].as_bool().unwrap(),
                "{key}: {value:?}"
            );
        }
    };
    check("evidence_class", &|x| accepts::<EvidenceClass>(x));
    check("confidence", &|x| accepts::<Confidence>(x));
    check("finding_status", &|x| accepts::<FindingStatus>(x));
    check("compatibility_status", &|x| {
        accepts::<CompatibilityStatus>(x)
    });
    check("provenance", &|x| accepts::<Provenance>(x));
}

#[test]
fn strict_record_decoding_matches_vectors() {
    for case in list(&vectors(), "records") {
        let json = s(&case["json"]);
        let ok = match s(&case["kind"]) {
            "evidence" => serde_json::from_str::<Evidence>(json).is_ok(),
            "declaration" => serde_json::from_str::<Declaration>(json).is_ok(),
            "finding" => serde_json::from_str::<Finding>(json).is_ok(),
            "fixture" => serde_json::from_str::<Fixture>(json).is_ok(),
            other => panic!("unknown kind {other}"),
        };
        let expected = s(&case["expect"]) == "ok";
        assert_eq!(ok, expected, "{} / {}", s(&case["kind"]), s(&case["name"]));
    }
}
