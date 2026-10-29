use super::*;

pub(crate) fn run_audit_sources(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &["strict"])?;
    let manifest = flags.string("manifest", "");
    let strict = flags.bool("strict");
    if manifest.is_empty() {
        bail!("--manifest is required");
    }
    let sources = load_sources(&manifest)?;
    let mut problems = Vec::new();
    for (index, item) in sources.iter().enumerate() {
        problems.extend(audit_source(item, index));
    }
    println!("Sources: {}", sources.len());
    if problems.is_empty() {
        println!("No source metadata problems found.");
        return Ok(());
    }
    println!("Problems: {}", problems.len());
    for problem in &problems {
        println!("- {problem}");
    }
    if strict {
        bail!("source audit failed");
    }
    Ok(())
}

pub(crate) fn run_download_sources(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &["include-unknown", "dry-run"])?;
    let manifest = flags.string("manifest", "");
    let out_dir = flags.string("out-dir", "");
    let allow_status = flags.string("allow-status", &sorted_open_access_statuses().join(","));
    let include_unknown = flags.bool("include-unknown");
    let dry_run = flags.bool("dry-run");
    let max_mb = flags.int("max-mb", 100)?;
    let timeout_seconds = flags.int("timeout", 30)?;
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();

    if manifest.is_empty() {
        bail!("--manifest is required");
    }
    if out_dir.is_empty() {
        bail!("--out-dir is required");
    }
    if max_mb <= 0 {
        bail!("--max-mb must be positive");
    }
    if timeout_seconds <= 0 {
        bail!("--timeout must be positive");
    }

    let sources = load_sources(&manifest)?;
    fs::create_dir_all(&out_dir).with_context(|| format!("create {out_dir}"))?;
    let allowed = parse_status_set(&allow_status);
    let max_bytes = i64::from(max_mb) * 1024 * 1024;
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_seconds as u64))
        .build()?;
    let log_path = Path::new(&out_dir).join("download-log.jsonl");
    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("open {}", log_path.display()))?;
    let mut downloaded_outputs = Vec::new();

    for (index, item) in sources.iter().enumerate() {
        let mut record = DownloadRecord {
            title: source_identity(item, index),
            url: source_url(item),
            status: "skipped".to_string(),
            ..DownloadRecord::default()
        };
        let (ok, reason) = can_download(item, &allowed, include_unknown);
        if !ok {
            record.reason = reason.clone();
            write_download_record(&mut log_file, &record)?;
            println!("SKIP {}: {reason}", record.title);
            continue;
        }
        if !is_http_url(&record.url) {
            record.reason = "missing http(s) url".to_string();
            write_download_record(&mut log_file, &record)?;
            println!("SKIP {}: missing http(s) url", record.title);
            continue;
        }
        if dry_run {
            record.status = "dry-run".to_string();
            write_download_record(&mut log_file, &record)?;
            println!("DRY {}: {}", record.title, record.url);
            continue;
        }
        match download_one(
            &client,
            &record.url.clone(),
            &record.title.clone(),
            &out_dir,
            max_bytes,
            &mut record,
        ) {
            Ok(()) => {
                write_download_record(&mut log_file, &record)?;
                if !record.path.trim().is_empty() {
                    downloaded_outputs.push(run_file("downloaded-source", record.path.as_str()));
                }
                println!("DOWNLOADED {}: {}", record.title, record.path);
            }
            Err(err) => {
                record.status = "error".to_string();
                record.reason = err.to_string();
                write_download_record(&mut log_file, &record)?;
                println!("ERROR {}: {err}", record.title);
            }
        }
    }
    println!("Wrote log: {}", log_path.display());
    let mut output_files = vec![run_file("download-log", log_path)];
    output_files.extend(downloaded_outputs);
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "download-sources",
        args,
        stage: if dry_run {
            "download-dry-run"
        } else {
            "download"
        },
        profile_version: "unknown".to_string(),
        started_at,
        inputs: input_file("source-manifest", &manifest),
        outputs: output_files,
        semantic_payloads: Vec::new(),
        network: !dry_run,
    })?;
    Ok(())
}

pub(crate) fn run_source_template(args: &[String]) -> Result<()> {
    let flags = parse_flags(args, &[])?;
    let output = flags.string("output", "");
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();
    if output.is_empty() {
        bail!("--output is required");
    }
    write_sources_csv(&output)?;
    println!("Wrote {output}");
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "source-template",
        args,
        stage: "source-template",
        profile_version: "unknown".to_string(),
        started_at,
        inputs: Vec::new(),
        outputs: output_file("source-manifest-template", &output),
        semantic_payloads: Vec::new(),
        network: false,
    })?;
    Ok(())
}

pub(crate) fn run_ingest(args: &[String]) -> Result<()> {
    let Some(command) = args.first() else {
        bail!(
            "ingest subcommand is required; usage: sok ingest last --sources \
            <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl>"
        );
    };
    match command.as_str() {
        "-h" | "--help" | "help" => {
            print_ingest_usage();
            Ok(())
        }
        "last" => run_ingest_last(&args[1..]),
        command => bail!("unknown ingest command {command:?}"),
    }
}

pub(crate) fn print_ingest_usage() {
    println!(
        r#"SoK ingest commands

Usage:
  sok ingest last --sources <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl> [--run-manifest <sok-run.json>]

Commands:
  last   Normalize the current run's source manifest into cataloged evidence JSONL.

The last command is bounded to the named input manifest and overwrites --output."#
    );
}

pub(crate) fn run_ingest_last(args: &[String]) -> Result<()> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_ingest_last_usage();
        return Ok(());
    }

    let flags = parse_flags(args, &[])?;
    let sources = flags.string("sources", "");
    let manifest = flags.string("manifest", "");
    let output = flags.string("output", "");
    let run_manifest_path = flags.string("run-manifest", "");
    let started_at = run_manifest::timestamp_now();

    if !sources.is_empty() && !manifest.is_empty() && sources != manifest {
        bail!("use only one of --sources or --manifest");
    }
    let input = first_non_empty([&sources, &manifest]);
    if input.is_empty() {
        bail!("--sources is required (or --manifest)");
    }
    if output.is_empty() {
        bail!("--output is required");
    }

    let normalized = ingest_last_manifest(&input, &output)?;
    println!(
        "Wrote {} cataloged evidence entries to {output}",
        normalized.evidence.len()
    );
    if !normalized.diagnostics.is_empty() {
        println!("Diagnostics: {}", normalized.diagnostics.len());
        for check in normalized.diagnostics {
            println!(
                "- {} {}: {}",
                diagnostic_severity_label(check.severity),
                check.check_id,
                check.message
            );
        }
    }
    write_optional_run_manifest(OptionalRunManifest {
        path: &run_manifest_path,
        command: "ingest last",
        args,
        stage: "ingest",
        profile_version: "unknown".to_string(),
        started_at,
        inputs: input_file("source-manifest", &input),
        outputs: output_file("evidence-jsonl", &output),
        semantic_payloads: Vec::new(),
        network: false,
    })?;
    Ok(())
}

pub(crate) fn ingest_last_manifest(
    input: &str,
    output: &str,
) -> Result<report::NormalizedSourceManifest> {
    let normalized = report::normalize_source_manifest(input)?;
    report::write_jsonl_file(output, &normalized.evidence)?;
    Ok(normalized)
}

pub(crate) fn print_ingest_last_usage() {
    println!(
        r#"Usage:
  sok ingest last --sources <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl> [--run-manifest <sok-run.json>]
  sok ingest last --manifest <sources.csv|sources.tsv|sources.json> --output <evidence.jsonl> [--run-manifest <sok-run.json>]

Writes one cataloged EvidenceEntry JSON object per source row.
The command does not download, crawl, browse, or append to any global ledger.
The requested output file is overwritten."#
    );
}
