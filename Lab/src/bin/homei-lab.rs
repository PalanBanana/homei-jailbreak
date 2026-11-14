//! `homei-lab`: the host controller for the virtual research lab.
//!
//! Every command is explicit. Nothing starts a virtual machine unless `launch` is given
//! `--start-vm`, nothing installs or removes a package without `--approve`, and nothing deletes a
//! snapshot or discards an overlay without `--confirm`. It never touches a physical device and
//! never uses `sudo`.

use ed25519_dalek::{Signer, SigningKey};
use homei_core::experiment::{ArtifactHash, BackendKind, BackendRef, ExperimentStatus, TargetRef};
use homei_core::labstate::{GuestSummary, InstalledPackage};
use homei_core::target::TargetManifest;
use homei_lab::backend::{self, plan_launch, preflight, DEFAULT_MEMORY_GIB};
use homei_lab::bootstrap::{self, PackageJob};
use homei_lab::experiments::{ExperimentSpec, ExperimentStore};
use homei_lab::firmware::{parse_artifact_hashes, to_public};
use homei_lab::ghidra::Inspector;
use homei_lab::guard::GuestRoot;
use homei_lab::hash::{sha256_hex, to_hex};
use homei_lab::host;
use homei_lab::packages::*;
use homei_lab::process::RealRunner;
use homei_lab::state::{build_state, empty_guest, StateInputs};
use homei_lab::storage::Workspace;
use homei_lab::timeutil::utc_now;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const USAGE: &str = "homei-lab: virtual research lab host controller

  target       [--repo-root DIR]                      validate the canonical target manifest
  preflight    [--firmware DIR] [--qemu PATH] [--memory-gib N] [--hash]
                                                      inspect the environment, starts nothing
  plan         [--firmware DIR] [--qemu PATH] [--memory-gib N]
                                                      print the launch command (dry run)
  launch       --firmware DIR --start-vm [--qemu PATH] [--timeout SECS]
                                                      start a VM. Refused unless preflight passes
  workspace    init|usage|snapshot|restore|delete-snapshot|discard DIR [...]
  bootstrap    WORKSPACE [--repo DIR --install ID --trust HEX --approve [--allow-unsigned]]
  pkg          sample-repo DIR | list --repo DIR | installed WS | manifest WS
               install WS --repo DIR --id ID --trust HEX --approve
               remove WS --id ID --approve | rollback WS --approve
  demo         --workspace DIR [--repo-root DIR]      run and record a real guest-workspace session
  export-state --repo-root DIR --lab-dir DIR --out FILE [--workspace DIR]
  analyze      --ipsw FILE --input DIR --out DIR [--ghidra FILE.gpr] [--date YYYY-MM-DD]
                                                      static firmware analysis; reads only, writes the report
  feasibility  --analysis FILE.json [--out DIR] [--date YYYY-MM-DD]
                                                      T8130 feasibility matrix from the analysis; reads only

Never runs a guest automatically. Never touches a device and needs no elevated privileges.";

/// The test-only signing key for the sample repository. It is public on purpose and signs only fixtures.
const SAMPLE_SEED: [u8; 32] = [0x42; 32];
const TARGET_FILE: &str = "Research/Targets/iPhone16,2-iOS27.0.1.json";
const GHIDRA_PROJECT: &str = "HOMEI_A17Pro_iOS27_0_1_24A446";
const KERNEL_REFERENCE_SHA256: &str =
    "71d3232aff357cf0e26d7252230cbcbc366b1f48d902de3d7fa5c13feea6f243";

fn opt(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn die(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn load_target(repo_root: &Path) -> TargetManifest {
    let path = repo_root.join(TARGET_FILE);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| die("cannot read the target manifest; use --repo-root"));
    let m: TargetManifest = serde_json::from_str(&text)
        .unwrap_or_else(|e| die(&format!("target manifest does not parse: {e}")));
    if let Err(e) = m.validate() {
        die(&format!("target manifest is invalid: {e}"));
    }
    m
}

fn qemu_path(args: &[String]) -> String {
    opt(args, "--qemu").unwrap_or_else(|| "qemu-system-aarch64".to_string())
}

fn target_ref(t: &TargetManifest) -> TargetRef {
    TargetRef {
        target_id: t.target_id.clone(),
        product_type: t.product_type.clone(),
        soc: t.soc.clone(),
        board: t.board.clone(),
        build_version: t.build_version.clone(),
    }
}

fn none_backend() -> BackendRef {
    BackendRef {
        name: "host workspace (no virtual machine)".to_string(),
        kind: BackendKind::None,
        version: None,
        qemu_version: None,
    }
}

fn sample_key() -> SigningKey {
    SigningKey::from_bytes(&SAMPLE_SEED)
}

fn sample_trust() -> String {
    to_hex(&sample_key().verifying_key().to_bytes())
}

fn sample_package(id: &str, deps: &[(&str, &str)], file: &str, body: &[u8]) -> Vec<u8> {
    let header = PackageHeader {
        format: 1,
        id: id.to_string(),
        version: "1.0.0".to_string(),
        description: format!(
            "Sample fixture package {id}. It contains inert text and is not a real research tool."
        ),
        license: "MIT".to_string(),
        source: "HOMEI sample fixtures".to_string(),
        depends: deps
            .iter()
            .map(|(i, r)| Dependency {
                id: i.to_string(),
                requirement: r.to_string(),
            })
            .collect(),
        conflicts: Vec::new(),
        scripts: Vec::new(),
        files: vec![PackageFile {
            path: file.to_string(),
            size: body.len() as u64,
            sha256: sha256_hex(body),
            executable: false,
        }],
    };
    build_archive(&header, &[body.to_vec()])
}

fn write_sample_repo(dir: &Path) -> PackageIndex {
    std::fs::create_dir_all(dir).unwrap_or_else(|_| die("cannot create the repository directory"));
    let key = sample_key();
    let archives = [
        sample_package(
            "homei-sample-lib",
            &[],
            "lib/README.txt",
            b"Inert sample library file. Not a real tool.\n",
        ),
        sample_package(
            "homei-sample-tool",
            &[("homei-sample-lib", ">=1.0")],
            "bin/sample-tool.txt",
            b"Inert sample tool file. It is never executed by HOMEI.\n",
        ),
    ];
    let mut packages = Vec::new();
    for bytes in archives {
        let tmp = index_entry_for("x", &bytes, None, "")
            .unwrap_or_else(|_| die("sample package is invalid"));
        let name = format!("{}-{}.hpk", tmp.id, tmp.version);
        let sig = to_hex(
            &key.sign(&signing_message(&tmp.id, &tmp.version, &tmp.sha256))
                .to_bytes(),
        );
        let entry = index_entry_for(
            &name,
            &bytes,
            Some(sig),
            "sample fixture signed with the public TEST-ONLY key",
        )
        .unwrap_or_else(|_| die("sample package is invalid"));
        std::fs::write(dir.join(&name), &bytes)
            .unwrap_or_else(|_| die("cannot write a sample archive"));
        packages.push(entry);
    }
    let index = PackageIndex {
        schema_version: 1,
        source_name: "HOMEI sample fixtures (test-only key)".to_string(),
        packages,
    };
    std::fs::write(
        dir.join("index.json"),
        serde_json::to_vec_pretty(&index).unwrap_or_default(),
    )
    .unwrap_or_else(|_| die("cannot write the index"));
    index
}

fn read_repo(dir: &Path) -> (PackageIndex, PathBuf) {
    let text = std::fs::read_to_string(dir.join("index.json"))
        .unwrap_or_else(|_| die("cannot read index.json in the repository"));
    (
        serde_json::from_str(&text)
            .unwrap_or_else(|e| die(&format!("index.json does not parse: {e}"))),
        dir.to_path_buf(),
    )
}

fn open_pm(ws: &Path) -> PackageManager {
    let w = Workspace::init(ws, &utc_now()).unwrap_or_else(|e| die(&e.to_string()));
    PackageManager::open(
        GuestRoot::open(w.overlay().host_path()).unwrap_or_else(|e| die(&e.to_string())),
    )
    .unwrap_or_else(|e| die(&format!("{e}")))
}

fn requests(args: &[String]) -> Vec<Request> {
    let id = opt(args, "--id")
        .or_else(|| opt(args, "--install"))
        .unwrap_or_else(|| die("--id is required"));
    vec![Request {
        id,
        requirement: opt(args, "--requirement").unwrap_or_else(|| "*".to_string()),
    }]
}

fn require_approval(args: &[String]) -> Approval {
    if !flag(args, "--approve") {
        die("this changes the guest. Re-run with --approve to confirm");
    }
    Approval::user_approved()
}

fn trust_from(args: &[String]) -> TrustRoot {
    let mut t = TrustRoot::empty();
    for i in 0..args.len() {
        if args[i] == "--trust" {
            let hex = args
                .get(i + 1)
                .unwrap_or_else(|| die("--trust needs a hex key"));
            t = t
                .with_key_hex(hex)
                .unwrap_or_else(|_| die("not a valid 32-byte Ed25519 public key in hex"));
        }
    }
    t
}

fn cmd_target(args: &[String]) {
    let root = PathBuf::from(opt(args, "--repo-root").unwrap_or_else(|| ".".into()));
    let m = load_target(&root);
    println!("target {} is valid", m.target_id);
    println!(
        "  {} / {} / {} / board {} / CPID {} / BDID {} / {} / iOS {} ({})",
        m.commercial_model,
        m.product_type,
        m.soc,
        m.board,
        m.cpid,
        m.bdid,
        m.architecture,
        m.os_version,
        m.build_version
    );
    println!(
        "  reference vm_status: {}, hardware fidelity: {:?}",
        m.vm_status.as_str(),
        m.hardware_fidelity
    );
    for d in &m.discrepancies {
        println!("  discrepancy {}: {}", d.id, d.summary);
    }
}

fn run_preflight(args: &[String]) -> (backend::PreflightReport, homei_core::labstate::HostSummary) {
    let fw = PathBuf::from(opt(args, "--firmware").unwrap_or_else(|| "firmware".into()));
    let qemu = qemu_path(args);
    let host = host::detect(&qemu, &RealRunner, Some(Path::new("/opt/ghidra")));
    let requested = opt(args, "--memory-gib")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MEMORY_GIB);
    (
        preflight(
            &RealRunner,
            &qemu,
            &fw,
            host.memory_available_gib,
            requested,
            flag(args, "--hash"),
        ),
        host,
    )
}

fn cmd_preflight(args: &[String]) {
    let (r, host) = run_preflight(args);
    println!(
        "host: {} {} | memory {} GiB available of {} GiB | KVM device present: {}",
        host.distribution,
        host.architecture,
        host.memory_available_gib,
        host.memory_total_gib,
        host.kvm_device_present
    );
    println!(
        "qemu: found={} version={:?} accelerators={:?}",
        r.qemu_found, r.qemu_version, r.accelerators
    );
    println!("darwin machine available: {}", r.darwin_machine_available);
    for f in &r.firmware {
        println!(
            "  firmware {:<12} present={:<5} {}",
            f.name, f.present, f.role
        );
    }
    println!("can launch: {}", r.can_launch());
    for b in &r.blockers {
        println!("  blocker: {b}");
    }
    println!("nothing was started.");
}

fn cmd_plan(args: &[String]) {
    let fw = opt(args, "--firmware").unwrap_or_else(|| "firmware".into());
    let mem = opt(args, "--memory-gib")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MEMORY_GIB);
    let plan = plan_launch(&qemu_path(args), &fw, mem, 600);
    println!("DRY RUN, nothing is started:\n{}", plan.command_line());
    for n in &plan.notes {
        println!("  note: {n}");
    }
}

fn cmd_launch(args: &[String]) {
    let (pre, _host) = run_preflight(args);
    let fw = opt(args, "--firmware").unwrap_or_else(|| die("--firmware is required"));
    let mem = opt(args, "--memory-gib")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MEMORY_GIB);
    let timeout = opt(args, "--timeout")
        .and_then(|v| v.parse().ok())
        .unwrap_or(600);
    let plan = plan_launch(&qemu_path(args), &fw, mem, timeout);
    if !flag(args, "--start-vm") {
        println!(
            "Launch refused: --start-vm was not given. A VM is never started implicitly.\n{}",
            plan.command_line()
        );
        std::process::exit(2);
    }
    let dtree = BTreeMap::new();
    match backend::launch(&RealRunner, &plan, &pre, None, &dtree) {
        Err(e) => die(&e),
        Ok(r) => {
            println!(
                "process status: {:?}, outcome: {:?}, kernel banner: {:?}, shell reached: {}",
                r.status,
                r.observation.outcome,
                r.observation.guest_kernel_version,
                r.observation.reached_shell
            );
            println!("The device tree identity was not read, so this outcome alone never makes the guest an exact target.");
        }
    }
}

fn cmd_workspace(args: &[String]) {
    let (sub, dir) = (
        args.get(1).map(String::as_str).unwrap_or(""),
        args.get(2)
            .unwrap_or_else(|| die("a workspace directory is required")),
    );
    let now = utc_now();
    let w = Workspace::init(Path::new(dir), &now).unwrap_or_else(|e| die(&e.to_string()));
    match sub {
        "init" => println!(
            "workspace ready: overlay is empty={}",
            w.overlay_files().map(|f| f.is_empty()).unwrap_or(false)
        ),
        "usage" => {
            let u = w.usage().unwrap_or_else(|e| die(&e.to_string()));
            println!(
                "overlay {} bytes, {} snapshot(s), {} bytes in snapshots",
                u.overlay_bytes, u.snapshots, u.snapshot_bytes
            );
        }
        "snapshot" => {
            let s = w
                .snapshot(args.get(3).map(String::as_str).unwrap_or("snapshot"), &now)
                .unwrap_or_else(|e| die(&e.to_string()));
            println!(
                "created {} ({} files, tree {})",
                s.id,
                s.files.len(),
                &s.tree_sha256[..16]
            );
        }
        "restore" => {
            w.restore(
                args.get(3)
                    .unwrap_or_else(|| die("a snapshot id is required")),
            )
            .unwrap_or_else(|e| die(&e.to_string()));
            println!(
                "restored; overlay tree {}",
                &w.overlay_tree_hash().unwrap_or_default()[..16]
            );
        }
        "delete-snapshot" => {
            w.delete_snapshot(
                args.get(3)
                    .unwrap_or_else(|| die("a snapshot id is required")),
                flag(args, "--confirm"),
            )
            .unwrap_or_else(|e| die(&e.to_string()));
            println!("snapshot deleted");
        }
        "discard" => {
            w.discard_overlay(flag(args, "--confirm"))
                .unwrap_or_else(|e| die(&e.to_string()));
            println!("overlay discarded");
        }
        _ => die("unknown workspace command"),
    }
}

fn cmd_bootstrap(args: &[String]) {
    let ws = args
        .get(1)
        .unwrap_or_else(|| die("a workspace directory is required"));
    let w = Workspace::init(Path::new(ws), &utc_now()).unwrap_or_else(|e| die(&e.to_string()));
    let repo = opt(args, "--repo").map(|r| read_repo(Path::new(&r)));
    let trust = trust_from(args);
    let fetch_dir = repo.as_ref().map(|(_, d)| d.clone());
    let fetch = move |e: &IndexEntry| -> Result<Vec<u8>, PkgError> {
        std::fs::read(fetch_dir.clone().unwrap_or_default().join(&e.archive))
            .map_err(|_| PkgError::NotInIndex(e.archive.clone()))
    };
    let job = repo.as_ref().map(|(index, _)| PackageJob {
        index,
        requests: requests(args),
        fetch: &fetch,
        trust: &trust,
        options: InstallOptions {
            allow_unsigned: flag(args, "--allow-unsigned"),
            abort_after_packages: None,
        },
        approval: require_approval(args),
    });
    let r = bootstrap::run(w.overlay(), &utc_now(), job);
    println!(
        "bootstrap {:?}: {} item(s) created, state {}",
        r.outcome,
        r.created.len(),
        r.manifest.state
    );
    if let Some(e) = &r.error {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn cmd_pkg(args: &[String]) {
    let sub = args.get(1).map(String::as_str).unwrap_or("");
    let target = args
        .get(2)
        .unwrap_or_else(|| die("a directory is required"));
    match sub {
        "sample-repo" => {
            let idx = write_sample_repo(Path::new(target));
            println!(
                "wrote {} sample package(s). TEST-ONLY trust key (public): {}",
                idx.packages.len(),
                sample_trust()
            );
        }
        "list" => {
            let (idx, _) = read_repo(Path::new(
                &opt(args, "--repo").unwrap_or_else(|| target.clone()),
            ));
            for p in &idx.packages {
                println!(
                    "{} {} signed={} depends={:?}",
                    p.id,
                    p.version,
                    p.signature.is_some(),
                    p.depends
                        .iter()
                        .map(|d| format!("{} {}", d.id, d.requirement))
                        .collect::<Vec<_>>()
                );
            }
        }
        "installed" => {
            let pm = open_pm(Path::new(target));
            for r in pm.installed().unwrap_or_default() {
                println!(
                    "{} {} signature={:?} provenance={:?} files={}",
                    r.id,
                    r.version,
                    r.signature,
                    r.provenance,
                    r.files.len()
                );
            }
        }
        "manifest" => println!(
            "{}",
            serde_json::to_string_pretty(
                &open_pm(Path::new(target))
                    .export_manifest()
                    .unwrap_or_else(|e| die(&format!("{e}")))
            )
            .unwrap_or_default()
        ),
        "install" => {
            let approval = require_approval(args);
            let (idx, dir) = read_repo(Path::new(
                &opt(args, "--repo").unwrap_or_else(|| die("--repo is required")),
            ));
            let mut pm = open_pm(Path::new(target));
            let fetch = move |e: &IndexEntry| -> Result<Vec<u8>, PkgError> {
                std::fs::read(dir.join(&e.archive))
                    .map_err(|_| PkgError::NotInIndex(e.archive.clone()))
            };
            let tx = pm
                .install(
                    approval,
                    &idx,
                    &requests(args),
                    &fetch,
                    &trust_from(args),
                    InstallOptions {
                        allow_unsigned: flag(args, "--allow-unsigned"),
                        abort_after_packages: None,
                    },
                    &utc_now(),
                )
                .unwrap_or_else(|e| die(&format!("{e}")));
            println!("{} committed: {} package(s)", tx.id, tx.ops.len());
        }
        "remove" => {
            let tx = open_pm(Path::new(target))
                .remove(
                    require_approval(args),
                    &opt(args, "--id").unwrap_or_else(|| die("--id is required")),
                    &utc_now(),
                )
                .unwrap_or_else(|e| die(&format!("{e}")));
            println!("{} committed", tx.id);
        }
        "rollback" => {
            let tx = open_pm(Path::new(target))
                .rollback_last(require_approval(args))
                .unwrap_or_else(|e| die(&format!("{e}")));
            println!("{} rolled back", tx.id);
        }
        _ => die("unknown pkg command"),
    }
}

/// Run a real guest-workspace session and record it as experiments. Nothing here starts a VM.
fn cmd_demo(args: &[String]) {
    let root = PathBuf::from(opt(args, "--repo-root").unwrap_or_else(|| ".".into()));
    let ws_dir =
        PathBuf::from(opt(args, "--workspace").unwrap_or_else(|| die("--workspace is required")));
    let target = load_target(&root);
    let now = utc_now();
    let ws = Workspace::init(&ws_dir, &now).unwrap_or_else(|e| die(&e.to_string()));
    let store = ExperimentStore::open(
        GuestRoot::open(ws.root().host_path()).unwrap_or_else(|e| die(&e.to_string())),
    )
    .unwrap_or_else(|e| die(&format!("{e}")));

    // Experiment 1: the feasibility preflight of this host. It runs --version, -accel help and -M help only.
    let (pre, host) = run_preflight(&[]);
    let a = store
        .plan(
            ExperimentSpec {
                kind: "feasibility-preflight".to_string(),
                target: target_ref(&target),
                backend: BackendRef { name: "darwin-vm (qemu-sptm) preflight".to_string(), kind: BackendKind::None, version: host.qemu_version.clone(), qemu_version: host.qemu_version.clone() },
                firmware_artifacts: Vec::new(),
                config: BTreeMap::from([("qemu".into(), "qemu-system-aarch64".into()), ("memory_gib".into(), DEFAULT_MEMORY_GIB.to_string()), ("firmware_dir".into(), "firmware".into())]),
                inputs: vec!["host environment".to_string()],
                limitations: vec!["The preflight starts no virtual machine. It says what is missing, not whether a guest would boot.".to_string()],
                snapshot_id: None,
            },
            &now,
        )
        .unwrap_or_else(|e| die(&format!("{e}")));
    store
        .start(&a.experiment_id)
        .unwrap_or_else(|e| die(&format!("{e}")));
    let summary = format!(
        "can_launch={}; darwin_machine_available={}; blockers: {}",
        pre.can_launch(),
        pre.darwin_machine_available,
        pre.blockers.join(" | ")
    );
    store
        .attach_log(
            &a.experiment_id,
            "preflight.json",
            serde_json::to_string_pretty(&pre)
                .unwrap_or_default()
                .as_bytes(),
        )
        .unwrap_or_else(|e| die(&format!("{e}")));
    store
        .finish(
            &a.experiment_id,
            ExperimentStatus::Completed,
            None,
            &summary,
            None,
        )
        .unwrap_or_else(|e| die(&format!("{e}")));
    println!("{}: {summary}", a.experiment_id);

    // Experiment 2: bootstrap, package lifecycle, snapshot and restore in the guest overlay.
    let sample_dir = ws_dir.join("sample-repo");
    write_sample_repo(&sample_dir);
    let (index, _) = read_repo(&sample_dir);
    let b = store
        .plan(
            ExperimentSpec {
                kind: "guest-workspace-lifecycle".to_string(),
                target: target_ref(&target),
                backend: none_backend(),
                firmware_artifacts: vec![ArtifactHash { name: "sample repository index".to_string(), sha256: sha256_hex(&serde_json::to_vec(&index).unwrap_or_default()) }],
                config: BTreeMap::from([("bootstrap".into(), bootstrap::BOOTSTRAP_VERSION.into()), ("install".into(), "homei-sample-tool".into()), ("key".into(), "public TEST-ONLY sample key".into())]),
                inputs: vec!["sample fixture packages".to_string()],
                limitations: vec![
                    "Runs in a host-side guest workspace overlay. No virtual machine was started and no Darwin guest exists here.".to_string(),
                    "The packages are inert fixtures signed with a public test key. They are not research tools.".to_string(),
                ],
                snapshot_id: None,
            },
            &now,
        )
        .unwrap_or_else(|e| die(&format!("{e}")));
    store
        .start(&b.experiment_id)
        .unwrap_or_else(|e| die(&format!("{e}")));
    let outcome = (|| -> Result<String, String> {
        let trust = TrustRoot::empty()
            .with_key_hex(&sample_trust())
            .map_err(|e| format!("{e}"))?;
        let fetch = |e: &IndexEntry| -> Result<Vec<u8>, PkgError> {
            std::fs::read(sample_dir.join(&e.archive))
                .map_err(|_| PkgError::NotInIndex(e.archive.clone()))
        };
        let job = PackageJob {
            index: &index,
            requests: vec![Request {
                id: "homei-sample-tool".into(),
                requirement: "*".into(),
            }],
            fetch: &fetch,
            trust: &trust,
            options: InstallOptions::default(),
            approval: Approval::user_approved(),
        };
        let r = bootstrap::run(ws.overlay(), &now, Some(job));
        if r.outcome != bootstrap::Outcome::Fresh && r.outcome != bootstrap::Outcome::Upgraded {
            return Err(format!("bootstrap was {:?}: {:?}", r.outcome, r.error));
        }
        let again = bootstrap::run(ws.overlay(), &now, None);
        let snap = ws
            .snapshot("after-bootstrap", &now)
            .map_err(|e| e.to_string())?;
        let mut pm = PackageManager::open(
            GuestRoot::open(ws.overlay().host_path()).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("{e}"))?;
        let installed = pm.installed().map_err(|e| format!("{e}"))?.len();
        pm.remove(Approval::user_approved(), "homei-sample-tool", &now)
            .map_err(|e| format!("{e}"))?;
        let removed = pm.installed().map_err(|e| format!("{e}"))?.len();
        ws.restore(&snap.id).map_err(|e| e.to_string())?;
        let restored_ok = ws.overlay_tree_hash().map_err(|e| e.to_string())? == snap.tree_sha256;
        let pm = PackageManager::open(
            GuestRoot::open(ws.overlay().host_path()).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("{e}"))?;
        let bad = pm.verify_installed().map_err(|e| format!("{e}"))?;
        if !restored_ok
            || !bad.is_empty()
            || installed != 2
            || removed != 1
            || again.outcome != bootstrap::Outcome::Unchanged
        {
            return Err(format!("checks failed: restored_ok={restored_ok} installed={installed} removed={removed} repeat={:?} bad={bad:?}", again.outcome));
        }
        Ok(format!("bootstrap {:?}; repeat run {:?}; installed 2 packages (dependency first); removed 1; snapshot {} restored to tree {}; installed files verified", r.outcome, again.outcome, snap.id, &snap.tree_sha256[..16]))
    })();
    match outcome {
        Ok(text) => {
            store
                .finish(
                    &b.experiment_id,
                    ExperimentStatus::Completed,
                    None,
                    &text,
                    None,
                )
                .unwrap_or_else(|e| die(&format!("{e}")));
            println!("{}: {text}", b.experiment_id);
        }
        Err(text) => {
            store
                .finish(
                    &b.experiment_id,
                    ExperimentStatus::Failed,
                    None,
                    &text,
                    None,
                )
                .unwrap_or_else(|e| die(&format!("{e}")));
            die(&text);
        }
    }
}

fn guest_summary(ws: &Path) -> GuestSummary {
    let Ok(w) = Workspace::init(ws, &utc_now()) else {
        return empty_guest();
    };
    let pm = PackageManager::open(
        GuestRoot::open(w.overlay().host_path()).unwrap_or_else(|e| die(&e.to_string())),
    )
    .unwrap_or_else(|e| die(&format!("{e}")));
    let usage = w.usage().ok();
    let b = bootstrap::state(w.overlay());
    GuestSummary {
        filesystem_scope: "isolated guest overlay (not a device)".to_string(),
        overlay_present: true,
        base_image_sha256: w.base().map(|b| b.sha256.clone()),
        snapshots: usage.map(|u| u.snapshots).unwrap_or(0),
        vm_running: false,
        bootstrap_version: b.as_ref().map(|b| b.bootstrap_version.clone()),
        bootstrap_state: b
            .map(|b| b.state)
            .unwrap_or_else(|| "not provisioned".to_string()),
        packages: pm
            .installed()
            .unwrap_or_default()
            .into_iter()
            .map(|r| InstalledPackage {
                id: r.id,
                version: r.version,
                provenance: r.provenance,
                signature: format!("{:?}", r.signature),
                files: r.files.len() as u32,
            })
            .collect(),
        transactions: pm.transactions().map(|t| t.len() as u32).unwrap_or(0),
    }
}

fn date_of_file(p: &Path) -> Option<String> {
    let secs = std::fs::metadata(p)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(homei_lab::timeutil::date_of(&homei_lab::timeutil::iso_from_unix(secs)).to_string())
}

fn cmd_export(args: &[String]) {
    let root = PathBuf::from(opt(args, "--repo-root").unwrap_or_else(|| ".".into()));
    let lab = PathBuf::from(opt(args, "--lab-dir").unwrap_or_else(|| die("--lab-dir is required")));
    let out = opt(args, "--out").unwrap_or_else(|| die("--out is required"));
    let target = load_target(&root);
    let (pre, mut host) = run_preflight(&[]);
    // The public snapshot says only that the host is Linux, not which distribution or machine it is.
    host.distribution = "Linux".to_string();
    let build = target.build_version.clone();
    let hashes = std::fs::read_to_string(lab.join("analysis/artifact-hashes.json"))
        .unwrap_or_else(|_| die("cannot read analysis/artifact-hashes.json in --lab-dir"));
    let records = parse_artifact_hashes(&hashes, &build)
        .unwrap_or_else(|e| die(&format!("artifact hash file is not usable: {e:?}")));
    if records.is_empty() {
        die("the artifact hash file has no record for the target build");
    }
    let firmware: Vec<_> = records.iter().map(|r| to_public(r, &build)).collect();
    let inspector = Inspector {
        project_dir: lab.join("ghidra"),
        project_name: GHIDRA_PROJECT.to_string(),
        analysis_dir: lab.join("analysis"),
        exports_dir: lab.join("ghidra/exports"),
        logs_dir: lab.join("logs"),
    };
    let ghidra = inspector.inspect(
        host.ghidra_version.clone(),
        "kernelcache.release.iPhone16,2",
        "AARCH64:LE:64:AppleSilicon",
        date_of_file(&lab.join("analysis/function-counts-by-fileset-entry.json")),
    );
    let (guest, experiments) = match opt(args, "--workspace") {
        Some(w) => {
            let store = ExperimentStore::open(
                GuestRoot::open(Path::new(&w)).unwrap_or_else(|e| die(&e.to_string())),
            )
            .unwrap_or_else(|e| die(&format!("{e}")));
            (
                guest_summary(Path::new(&w)),
                store.list().unwrap_or_default(),
            )
        }
        None => (empty_guest(), Vec::new()),
    };
    let state = build_state(StateInputs {
        generated_on: homei_lab::timeutil::date_of(&utc_now()).to_string(),
        target,
        host,
        preflight: pre,
        reference_kernel_sha256: Some(KERNEL_REFERENCE_SHA256.to_string()),
        boot: None,
        firmware,
        ghidra,
        guest,
        experiments,
        correlations: Vec::new(),
    });
    if let Err(e) = state.validate() {
        die(&format!(
            "the lab state failed validation and was not written: {e}"
        ));
    }
    std::fs::write(&out, serde_json::to_vec_pretty(&state).unwrap_or_default())
        .unwrap_or_else(|_| die("cannot write the output file"));
    println!("wrote the lab state: VM status {}, {} firmware artifact(s), {} function(s) in the Ghidra summary, {} experiment(s)", state.fidelity.vm_status.as_str(), state.firmware.len(), state.ghidra.function_count, state.experiments.len());
}

/// Static firmware analysis. Reads the restore archive, the manifest and the extracted tree, and
/// writes a JSON bundle and a Markdown report to `--out`. Nothing is executed.
fn cmd_analyze(args: &[String]) {
    let ipsw = PathBuf::from(opt(args, "--ipsw").unwrap_or_else(|| die("--ipsw is required")));
    let input = PathBuf::from(opt(args, "--input").unwrap_or_else(|| die("--input is required")));
    let out = PathBuf::from(opt(args, "--out").unwrap_or_else(|| die("--out is required")));
    let ghidra = opt(args, "--ghidra").map(PathBuf::from);
    let date = opt(args, "--date").unwrap_or_else(|| utc_now()[..10].to_string());
    let bundle = homei_lab::analysis::research::analyze(&input, &ipsw, ghidra.as_deref(), &date)
        .unwrap_or_else(|e| die(&e.to_string()));
    std::fs::create_dir_all(&out).unwrap_or_else(|_| die("cannot create the output directory"));
    let json = serde_json::to_string_pretty(&bundle).unwrap_or_else(|_| die("cannot encode"));
    std::fs::write(out.join("t8130-firmware-analysis.json"), json + "\n")
        .unwrap_or_else(|_| die("cannot write the JSON bundle"));
    std::fs::write(
        out.join("t8130-firmware-analysis.md"),
        homei_lab::analysis::report::markdown(&bundle),
    )
    .unwrap_or_else(|_| die("cannot write the report"));
    println!(
        "analysis written: {} components, {} claims",
        bundle.components.len(),
        bundle.claims.len()
    );
}

/// Feasibility matrix. Implemented in the feasibility module; see that module for the statuses.
fn cmd_feasibility(args: &[String]) {
    let analysis =
        PathBuf::from(opt(args, "--analysis").unwrap_or_else(|| die("--analysis is required")));
    let out = PathBuf::from(opt(args, "--out").unwrap_or_else(|| ".".into()));
    let date = opt(args, "--date").unwrap_or_else(|| utc_now()[..10].to_string());
    let text =
        std::fs::read_to_string(&analysis).unwrap_or_else(|_| die("cannot read the analysis file"));
    let bundle: homei_lab::analysis::research::Bundle =
        serde_json::from_str(&text).unwrap_or_else(|e| {
            die(&format!(
                "analysis file does not match the bundle schema: {e}"
            ))
        });
    let matrix = homei_lab::feasibility::evaluate(&bundle, &date).unwrap_or_else(|e| die(&e));
    std::fs::create_dir_all(&out).unwrap_or_else(|_| die("cannot create the output directory"));
    let json = serde_json::to_string_pretty(&matrix).unwrap_or_else(|_| die("cannot encode"));
    std::fs::write(out.join("t8130-feasibility.json"), json + "\n")
        .unwrap_or_else(|_| die("cannot write"));
    std::fs::write(
        out.join("t8130-feasibility-report.md"),
        homei_lab::feasibility::markdown(&matrix),
    )
    .unwrap_or_else(|_| die("cannot write"));
    println!(
        "feasibility written: {} components",
        matrix.components.len()
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if flag(&args, "--help") || flag(&args, "-h") {
        println!("{USAGE}");
        return;
    }
    match args.first().map(String::as_str) {
        Some("target") => cmd_target(&args),
        Some("preflight") => cmd_preflight(&args),
        Some("plan") => cmd_plan(&args),
        Some("launch") => cmd_launch(&args),
        Some("workspace") => cmd_workspace(&args),
        Some("bootstrap") => cmd_bootstrap(&args),
        Some("pkg") => cmd_pkg(&args),
        Some("demo") => cmd_demo(&args),
        Some("export-state") => cmd_export(&args),
        Some("analyze") => cmd_analyze(&args),
        Some("feasibility") => cmd_feasibility(&args),
        _ => {
            println!("{USAGE}");
            std::process::exit(if args.is_empty() { 0 } else { 2 });
        }
    }
}
