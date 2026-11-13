//! `homei`: host-side command line view of the HOMEI core.
//!
//! Reads bundled data only. It has no device, network or filesystem-writing code.

use homei_bootstrap::{models::Manifest, planning, simulation};
use homei_catalog::Catalog;
use homei_core::{
    compatibility::{self, Decision},
    data::{self, Bundle},
    diagnostics, recovery, safety, PRODUCT_NAME, STAGE_NOTICE, TAGLINE,
};

const USAGE: &str = "usage: homei <command> [fixture-id]

commands:
  evaluate     [FIX-A|FIX-B|FIX-C]  compatibility decision for a test fixture
  capabilities                      status of every privileged capability
  validate                          validate the bundled data
  diagnostics  [fixture-id]         sanitized diagnostic report (JSON)
  catalog                           list catalog metadata (nothing is installable)
  simulate     [fixture-id]         simulated bootstrap plan (nothing is written)
  recovery     [fixture-id]         recovery topics and GO / NO-GO guidance
  authorize    <operation>          ask the Safety Manager about an operation";

fn decide(bundle: &Bundle, id: &str) -> Result<Decision, String> {
    let fixture = bundle
        .fixture(id)
        .ok_or_else(|| format!("unknown fixture {id}"))?;
    println!(
        "Profile: {} ({})",
        fixture.label,
        fixture.profile.provenance.label()
    );
    Ok(compatibility::evaluate(
        &fixture.profile,
        &bundle.compatibility,
        &bundle.evidence,
    ))
}

fn run() -> Result<bool, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");
    let arg = args.get(1).map(String::as_str);
    let bundle = data::bundled().map_err(|e| e.to_string())?;
    let fixture_id = arg.unwrap_or("FIX-A");
    println!("{PRODUCT_NAME}: {TAGLINE}\n{STAGE_NOTICE}\n");

    match cmd {
        "evaluate" => {
            let d = decide(&bundle, fixture_id)?;
            println!("Compatibility: {}", d.status.label());
            for line in &d.explanation {
                println!("  - {line}");
            }
            println!("{}", compatibility::RESEARCH_ONLY_LABEL);
        }
        "capabilities" => {
            for c in safety::privileged_capabilities() {
                println!("{:<24} {} ({})", c.name, c.state, c.reason);
            }
        }
        "validate" => {
            let checks = diagnostics::validate_bundle(&bundle);
            let mut all_ok = true;
            for c in &checks {
                println!("{} {}", if c.passed { "PASS" } else { "FAIL" }, c.name);
                all_ok &= c.passed;
            }
            let catalog = Catalog::bundled()?;
            let problems = catalog.validate();
            println!(
                "{} catalog metadata",
                if problems.is_empty() { "PASS" } else { "FAIL" }
            );
            for p in &problems {
                println!("  - {p}");
            }
            return Ok(all_ok && problems.is_empty());
        }
        "diagnostics" => {
            let d = decide(&bundle, fixture_id)?;
            let report = diagnostics::build_report(&bundle, &d);
            println!("{}", diagnostics::export(&report)?);
        }
        "catalog" => {
            let d = decide(&bundle, fixture_id)?;
            let catalog = Catalog::bundled()?;
            for p in &catalog.packages {
                let a = catalog.availability(p, &d);
                println!(
                    "{} {} [{}] licence {}",
                    p.id,
                    p.version,
                    if p.fixture { "fixture" } else { "package" },
                    p.license
                );
                println!("  {}", a.reason);
                println!("  dependencies: {:?}", catalog.dependency_summary(&p.id));
            }
        }
        "simulate" => {
            let d = decide(&bundle, fixture_id)?;
            let manifest = Manifest::bundled()?;
            let plan = planning::plan(&manifest, &["fixture.catalog-client"])
                .map_err(|e| e.to_string())?;
            let report = simulation::simulate(&plan, &d);
            for line in &report.status_lines {
                println!("{line}");
            }
        }
        "recovery" => {
            let d = decide(&bundle, fixture_id)?;
            for t in recovery::topics() {
                println!("{} [{:?}]\n  {}", t.title, t.status, t.body);
            }
            for s in recovery::signing_observations() {
                println!(
                    "Signing observation {} {} ({}, {}, verified: {})",
                    s.build, s.observation, s.source, s.date, s.verified
                );
            }
            let g = recovery::guidance(&d);
            println!("{:?}", g.verdict);
            for r in g.reasons {
                println!("  - {r}");
            }
        }
        "authorize" => {
            let name = arg.ok_or("authorize needs an operation name")?;
            let op = safety::Operation::parse(name);
            println!("{:?}", safety::authorize(&op));
        }
        _ => {
            println!("{USAGE}");
            return Ok(cmd.is_empty());
        }
    }
    Ok(true)
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(2),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
