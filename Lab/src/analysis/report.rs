//! Sanitized Markdown report for an analysis bundle.
//!
//! The report names files relative to the input root and never prints a local path, a device
//! identifier or an account detail. Every statement carries its confidence tag.

use super::research::{Availability, Bundle};
use std::fmt::Write;

fn tag_of(claim_confidence: &super::confidence::Confidence) -> &'static str {
    claim_confidence.tag()
}

fn short(hex: &str) -> String {
    hex.chars().take(16).collect::<String>() + "…"
}

fn availability_label(a: Availability) -> &'static str {
    match a {
        Availability::Analysed => "analysed",
        Availability::ContainerOnly => "container only",
        Availability::Encrypted => "encrypted (keys not available)",
        Availability::Unsupported => "unsupported format",
        Availability::Unavailable => "unavailable (no local copy)",
        Availability::Missing => "missing from archive",
        Availability::NoManifestPath => "no path stated in manifest",
    }
}

fn container_label(c: &Option<super::im4p::Container>) -> String {
    use super::im4p::Container;
    match c {
        None => "-".into(),
        Some(Container::Im4p(i)) => format!("IM4P `{}`", i.fourcc),
        Some(Container::Aea1) => "AEA1".into(),
        Some(Container::UdifDisk) => "UDIF disk image".into(),
        Some(Container::ApfsContainer) => "APFS container".into(),
        Some(Container::Unclassified) => "unclassified".into(),
    }
}

/// Render the bundle as Markdown.
pub fn markdown(b: &Bundle) -> String {
    let mut s = String::new();
    let t = &b.target;
    let _ = writeln!(s, "# T8130 firmware analysis\n");
    let _ = writeln!(
        s,
        "Target: **{}**. Analysed on {}. Schema {}. Tool `{}` {}.\n",
        t.label,
        b.provenance.analysed_on,
        b.schema_version,
        b.provenance.tool,
        b.provenance.tool_version
    );
    let _ = writeln!(
        s,
        "This is a static analysis of firmware files. Nothing was executed, booted or emulated. A passing check shows that the file names the target. It does not show that the target boots, runs or is compatible.\n"
    );

    s.push_str(
        "## Target identity\n\n| Check | Expected | Observed | Result |\n|---|---|---|---|\n",
    );
    for c in &b.identity_checks {
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} |",
            c.id,
            c.expected,
            c.observed,
            if c.passed { "PASS" } else { "FAIL" }
        );
    }
    s.push('\n');

    s.push_str("## Restore archive\n\n");
    let _ = writeln!(
        s,
        "- Entries in the central directory: {} ({} stored, {} deflated)",
        b.archive.entry_count, b.archive.stored_entries, b.archive.deflated_entries
    );
    let _ = writeln!(
        s,
        "- Uncompressed total: {} bytes (listing only; no entry was decompressed)",
        b.archive.uncompressed_total_bytes
    );
    let _ = writeln!(
        s,
        "- Erase components named by the manifest but absent from the archive: {}\n",
        b.archive.erase_components_missing_from_archive.len()
    );

    s.push_str("## Components (erase identity)\n\n");
    let sm = &b.summary;
    let _ = writeln!(
        s,
        "{} components: {} analysed, {} container only, {} encrypted, {} unsupported, {} unavailable, {} missing, {} with no path stated.\n",
        sm.erase_components, sm.analysed, sm.container_only, sm.encrypted, sm.unsupported, sm.unavailable, sm.missing, sm.no_manifest_path
    );
    s.push_str("| Component | Availability | Container | SHA-384 matches manifest | Payload SHA-256 |\n|---|---|---|---|---|\n");
    for c in &b.components {
        let digest = match c.manifest_digest_matches {
            Some(true) => "yes",
            Some(false) => "no",
            None => "not checked",
        };
        let payload = c
            .payload_sha256
            .as_deref()
            .map(short)
            .unwrap_or_else(|| "-".into());
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} | {} |",
            c.name,
            availability_label(c.availability),
            container_label(&c.container),
            digest,
            payload
        );
    }
    s.push('\n');

    if !b.security.is_empty() {
        s.push_str("## Security and coprocessor images\n\n| Role | Component | Availability | Encryption | Payload compression |\n|---|---|---|---|---|\n");
        for x in &b.security {
            let comp = x
                .payload_compression
                .map(|c| format!("{c:?}").to_lowercase())
                .unwrap_or_else(|| "-".into());
            let _ = writeln!(
                s,
                "| {} | {} | {} | {} | {} |",
                x.role,
                x.component,
                availability_label(x.availability),
                x.encryption,
                comp
            );
        }
        s.push('\n');
    }

    if let Some(k) = &b.kernelcache {
        s.push_str("## Kernelcache\n\n");
        let _ = writeln!(s, "- Payload: {} bytes", k.payload_bytes);
        match &k.macho {
            Ok(m) => {
                let _ = writeln!(
                    s,
                    "- Mach-O: {} {}, cputype {}, cpusubtype {}, {} load commands",
                    m.architecture, m.filetype, m.cputype, m.cpusubtype_hex, m.load_command_count
                );
                let _ = writeln!(
                    s,
                    "- Segments: {}",
                    m.segments
                        .iter()
                        .map(|g| g.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                let _ = writeln!(s, "- Fileset entries: {}", k.fileset_entry_count);
                if let (Some(p), Some(v)) = (m.build_platform, m.minimum_os.as_ref()) {
                    let _ = writeln!(s, "- Build platform {p}, minimum OS {v}");
                }
            }
            Err(e) => {
                let _ = writeln!(s, "- Mach-O parse failed: {e}");
            }
        }
        if let Some(x) = &k.xnu_version {
            let _ = writeln!(s, "- XNU version string: `{x}`");
        }
        if let Some(c) = &k.configuration_token {
            let _ = writeln!(s, "- Kernel configuration token: `{c}`");
        }
        s.push('\n');
    }

    if let Some(d) = &b.devicetree {
        s.push_str("## Device tree\n\n");
        let _ = writeln!(s, "- Nodes: {}", d.node_count);
        let _ = writeln!(s, "- Root compatible: {}", d.root_compatible.join(", "));
        let _ = writeln!(
            s,
            "- Root model: {}",
            d.root_model.clone().unwrap_or_else(|| "-".into())
        );
        let _ = writeln!(
            s,
            "- Target type: {}",
            d.root_target_type.clone().unwrap_or_else(|| "-".into())
        );
        let _ = writeln!(s, "- arm-io compatible: {}", d.arm_io_compatible.join(", "));
        for r in &d.arm_io_ranges {
            let _ = writeln!(
                s,
                "- arm-io range: child {} to parent {} size {}",
                r.child_base_hex, r.parent_base_hex, r.size_hex
            );
        }
        if let Some(a) = &d.aic {
            let _ = writeln!(s, "\n### Interrupt controller\n");
            let _ = writeln!(
                s,
                "- Path {}, compatible {}",
                a.path,
                a.compatible.join(", ")
            );
            let _ = writeln!(
                s,
                "- Interrupt cells: {}",
                a.interrupt_cells.map_or("-".into(), |c| c.to_string())
            );
            for r in &a.reg {
                let _ = writeln!(
                    s,
                    "- reg: address {}, size {} (relative to arm-io)",
                    r.address_hex, r.size_hex
                );
            }
            let _ = writeln!(s, "- reg-names present: {}", a.reg_names_present);
            match &a.translated_base_hex {
                Some(t) => {
                    let _ = writeln!(
                        s,
                        "- Translated base: {t} ({})",
                        a.translation_source.clone().unwrap_or_default()
                    );
                }
                None => {
                    let _ = writeln!(s, "- Translated base: not established");
                }
            }
            let _ = writeln!(
                s,
                "- Consumers with one interrupt cell: {}; with several elements: {}",
                a.consumers_single_cell, a.consumers_multi_element
            );
            let _ = writeln!(
                s,
                "- aic-iack-offset in the tree: {}",
                a.aic_iack_offset_in_tree
            );
        }
        if let Some(p) = &d.pmgr {
            let _ = writeln!(s, "\n### Power manager\n");
            let _ = writeln!(
                s,
                "- Path {}, compatible {}",
                p.path,
                p.compatible.join(", ")
            );
            let _ = writeln!(s, "- Register windows: {}", p.reg_windows);
            let _ = writeln!(
                s,
                "- Power-domain names found in the blob: {}",
                p.power_domain_names.len()
            );
        }
        s.push('\n');
    }

    if let Some(g) = &b.ghidra {
        s.push_str("## Ghidra project metadata\n\n");
        let _ = writeln!(s, "- Project: {}", g.project_name);
        let _ = writeln!(
            s,
            "- Project file: {} ({} bytes)",
            g.project_file_state, g.project_file_bytes
        );
        let _ = writeln!(
            s,
            "- Repository present: {}; index files scanned: {}",
            g.repository_present, g.index_files
        );
        for n in &g.names {
            let _ = writeln!(s, "- `{}` in index: {}", n.name, n.present_in_index);
        }
        let _ = writeln!(s, "- Function count: not exported (unknown)\n");
    }

    s.push_str(
        "## Claims\n\n| ID | Tag | Statement | Source | Limitation |\n|---|---|---|---|---|\n",
    );
    for c in &b.claims {
        let _ = writeln!(
            s,
            "| {} | [{}] | {} | {} | {} |",
            c.id,
            tag_of(&c.confidence),
            c.statement,
            c.source,
            c.limitation
        );
    }
    s.push('\n');

    s.push_str("## Provenance\n\n| Role | File | Bytes | SHA-256 |\n|---|---|---|---|\n");
    for i in &b.provenance.inputs {
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} |",
            i.role, i.file_name, i.bytes, i.sha256
        );
    }
    s.push('\n');
    for e in &b.provenance.external_steps {
        let _ = writeln!(s, "- External step: {e}");
    }
    s.push_str("\nTags: [T] read from a target artifact, [M] stated by the manifest, [I] inferred, [U] unknown.\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::confidence::Confidence;

    #[test]
    fn tags_are_the_documented_letters() {
        assert_eq!(tag_of(&Confidence::Target), "T");
        assert_eq!(tag_of(&Confidence::Manifest), "M");
        assert_eq!(tag_of(&Confidence::Inferred), "I");
        assert_eq!(tag_of(&Confidence::Unknown), "U");
    }
}
