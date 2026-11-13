use homei_bootstrap::models::{Component, ComponentKind, Manifest};
use homei_bootstrap::planning::{self, PlanError};
use homei_bootstrap::simulation::{self, StepOutcome};
use homei_catalog::{Catalog, Integrity, Package};
use homei_core::compatibility::{CompatibilityStatus, Decision};
use homei_core::device_profile::{Provenance, SocFamily};

fn pkg(id: &str, deps: &[&str]) -> Package {
    Package {
        id: id.into(),
        name: id.into(),
        version: "1.0.0".into(),
        summary: "synthetic".into(),
        source_id: "SRC-FIXTURE".into(),
        license: "MIT".into(),
        compatibility: vec![],
        integrity: Integrity {
            algorithm: "sha256".into(),
            digest: "0".repeat(64),
            verified: false,
        },
        dependencies: deps.iter().map(|s| s.to_string()).collect(),
        fixture: true,
    }
}

fn decision(status: CompatibilityStatus) -> Decision {
    Decision {
        status,
        soc_family: Some(SocFamily::A17Pro),
        matched_declaration: None,
        downgraded: false,
        profile_provenance: Provenance::TestFixture,
        explanation: vec![],
    }
}

#[test]
fn bundled_catalog_is_valid_and_all_fixture() {
    let c = Catalog::bundled().unwrap();
    assert_eq!(c.validate(), Vec::<String>::new());
    assert!(c
        .packages
        .iter()
        .all(|p| p.fixture && !p.integrity.verified));
    assert!(c
        .sources
        .iter()
        .all(|s| s.trust == "test_fixture" && s.url.is_none()));
}

#[test]
fn nothing_is_installable_and_install_is_explicitly_unsupported() {
    let c = Catalog::bundled().unwrap();
    let d = decision(CompatibilityStatus::Supported);
    for p in &c.packages {
        let a = c.availability(p, &d);
        assert!(!a.installable);
        assert!(a.reason.contains("cannot install anything"));
        assert!(c.install(&p.id).is_err());
    }
}

#[test]
fn dependency_summary_is_transitive_sorted_and_cycle_safe() {
    let c = Catalog::bundled().unwrap();
    assert_eq!(
        c.dependency_summary("homei.fixture.sample-tool"),
        ["homei.fixture.core-lib", "homei.fixture.sample-theme"]
    );
    assert!(c.dependency_summary("homei.fixture.core-lib").is_empty());
    assert!(c.dependency_summary("missing").is_empty());

    let cyclic = Catalog {
        packages: vec![pkg("a.one", &["a.two"]), pkg("a.two", &["a.one"])],
        sources: c.sources.clone(),
    };
    assert_eq!(cyclic.dependency_summary("a.one"), ["a.one", "a.two"]);
}

#[test]
fn validation_catches_bad_metadata() {
    let base = Catalog::bundled().unwrap();
    let mut bad_id = pkg("Bad Id!", &[]);
    bad_id.license = " ".into();
    let mut bad_digest = pkg("ok.digest", &[]);
    bad_digest.integrity.digest = "xyz".into();
    let mut unverified = pkg("ok.real", &[]);
    unverified.fixture = false;
    let mut bad_source = pkg("ok.source", &["does.not.exist"]);
    bad_source.source_id = "SRC-NOPE".into();
    let c = Catalog {
        packages: vec![bad_id, bad_digest, unverified, bad_source],
        sources: base.sources,
    };
    let problems = c.validate().join("\n");
    for needle in [
        "invalid package id",
        "licence is required",
        "sha256 hex digest",
        "needs verified integrity",
        "unknown source",
        "unknown dependency",
    ] {
        assert!(problems.contains(needle), "missing: {needle}\n{problems}");
    }
}

#[test]
fn bootstrap_plan_orders_dependencies_first() {
    let m = Manifest::bundled().unwrap();
    let plan = planning::plan(&m, &["fixture.catalog-client"]).unwrap();
    let ids: Vec<_> = plan.steps.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids.first(), Some(&"fixture.runtime"));
    assert_eq!(ids.last(), Some(&"fixture.catalog-client"));
    assert_eq!(ids.len(), 4);
    let pos = |id: &str| ids.iter().position(|x| *x == id).unwrap();
    assert!(pos("fixture.package-manager") < pos("fixture.catalog-client"));
    assert!(pos("fixture.core-tools") < pos("fixture.catalog-client"));
}

#[test]
fn bootstrap_plan_reports_unknown_components_and_cycles() {
    let m = Manifest::bundled().unwrap();
    assert_eq!(
        planning::plan(&m, &["nope"]),
        Err(PlanError::UnknownComponent("nope".into()))
    );
    let c = |id: &str, dep: &str| Component {
        id: id.into(),
        name: id.into(),
        version: "1".into(),
        kind: ComponentKind::Tool,
        depends_on: vec![dep.into()],
    };
    let cyclic = Manifest {
        schema_version: 1,
        note: String::new(),
        components: vec![c("a", "b"), c("b", "a")],
    };
    assert!(matches!(
        planning::plan(&cyclic, &["a"]),
        Err(PlanError::Cycle(_))
    ));
}

#[test]
fn simulation_is_blocked_unless_compatibility_is_positive_and_never_installable() {
    let plan = planning::plan(&Manifest::bundled().unwrap(), &["fixture.catalog-client"]).unwrap();
    for status in [
        CompatibilityStatus::NotEstablished,
        CompatibilityStatus::Unknown,
        CompatibilityStatus::Unsupported,
    ] {
        let r = simulation::simulate(&plan, &decision(status));
        assert!(r
            .steps
            .iter()
            .all(|s| s.outcome == StepOutcome::BlockedByCompatibility));
        assert_eq!(r.writes_performed, 0);
        assert!(!r.installable);
    }
    let r = simulation::simulate(&plan, &decision(CompatibilityStatus::Supported));
    assert!(r.steps.iter().all(|s| s.outcome == StepOutcome::Simulated));
    assert_eq!(r.writes_performed, 0);
    assert!(!r.system_changes);
    assert!(
        !r.installable,
        "even a positive simulation is not an installation"
    );
    assert_eq!(r.banner, simulation::BANNER);
}
