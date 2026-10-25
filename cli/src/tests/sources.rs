use super::*;

#[test]
fn access_audit_and_download_policy_match_contract() {
    let flat = Source {
        url: "https://example.test/source.pdf".to_string(),
        access_status: " Free_Web ".to_string(),
        budget_estimate: "$0".to_string(),
        ..Source::default()
    };
    let flat_access = source_access(&flat);
    assert_eq!(flat_access.status, "free_web");
    assert_eq!(flat_access.route, "https://example.test/source.pdf");

    let complete = Source {
        title: "Open review".to_string(),
        source_type: "paper".to_string(),
        url: "https://example.test/review.pdf".to_string(),
        access_status: "open_access".to_string(),
        access_route: "Official URL".to_string(),
        why_it_matters: "Sets the research frame.".to_string(),
        use_in_curriculum: "Read before the first module.".to_string(),
        ..Source::default()
    };
    assert!(audit_source(&complete, 0).is_empty());

    let missing = audit_source(&Source::default(), 0);
    for want in [
        "source-1: missing title/citation",
        "source-1: missing type",
        "source-1: missing identifier or url",
        "source-1: missing access status",
        "source-1: missing access route",
        "source-1: missing source role or relevance note",
    ] {
        assert!(
            missing.contains(&want.to_string()),
            "{missing:?} should contain {want}"
        );
    }

    let allowed = parse_status_set(&sorted_open_access_statuses().join(","));
    for status in [
        "open_access",
        " FREE_WEB ",
        "public_domain",
        "cc_by",
        "cc_by_sa",
        "official_open",
        "user_provided",
    ] {
        assert_eq!(
            can_download(
                &Source {
                    access_status: status.to_string(),
                    ..Source::default()
                },
                &allowed,
                false
            ),
            (true, "allowed".to_string())
        );
    }
    assert_eq!(
        can_download(
            &Source {
                access_status: "unknown".to_string(),
                ..Source::default()
            },
            &allowed,
            false,
        ),
        (false, "blocked access status: unknown".to_string())
    );
    assert_eq!(
        can_download(&Source::default(), &allowed, true),
        (true, "unknown allowed by flag".to_string())
    );
}

#[test]
fn filename_and_download_record_helpers_are_stable() {
    assert_eq!(
        sanitize_filename("Field Notes: 2026.pdf"),
        "Field-Notes-2026.pdf"
    );
    assert_eq!(
        sanitize_filename(" folder/report?.pdf "),
        "folder-report-.pdf"
    );
    assert_eq!(sanitize_filename("../../secret.pdf"), "secret.pdf");
    assert_eq!(sanitize_filename("***"), "source.bin");
    assert_eq!(common_extension("Application/PDF"), Some(".pdf"));

    assert_eq!(
        filename_from_headers(
            "https://example.test/ignored",
            Some(r#"attachment; filename="../../My Paper Final.pdf""#),
            Some("application/pdf"),
            "fallback",
        ),
        "My-Paper-Final.pdf"
    );
    assert_eq!(
        filename_from_headers(
            "https://example.test/files/report.pdf?download=1",
            None,
            Some("application/pdf"),
            "fallback",
        ),
        "report.pdf"
    );
    assert_eq!(
        filename_from_headers(
            "https://example.test/",
            None,
            Some("text/plain; charset=utf-8"),
            "Lecture Notes"
        ),
        "Lecture-Notes.txt"
    );
    assert_eq!(
        filename_from_headers("https://example.test/", None, None, "Untitled Source"),
        "Untitled-Source.bin"
    );

    let mut buf = Vec::new();
    write_download_record(
        &mut buf,
        &DownloadRecord {
            title: "Open paper".to_string(),
            url: "https://example.test/open.pdf".to_string(),
            status: "downloaded".to_string(),
            path: "/tmp/open.pdf".to_string(),
            bytes: 123,
            content_type: "application/pdf".to_string(),
            ..DownloadRecord::default()
        },
    )
    .unwrap();
    assert!(String::from_utf8(buf.clone()).unwrap().ends_with('\n'));
    let decoded: DownloadRecord = serde_json::from_slice(buf.split_last().unwrap().1).unwrap();
    assert_eq!(decoded.title, "Open paper");
    assert_eq!(decoded.bytes, 123);
}

#[test]
fn download_sources_writes_success_blocked_and_failed_records() {
    let Some(server) = spawn_server(2, |path| match path {
        "/open" => http_response(
            "200 OK",
            &[
                ("Content-Type", "text/plain"),
                (
                    "Content-Disposition",
                    r#"attachment; filename="../../Unsafe Notes.txt""#,
                ),
            ],
            b"open body",
        ),
        "/fail" => http_response(
            "500 Internal Server Error",
            &[("Content-Type", "text/plain")],
            b"failed",
        ),
        _ => http_response(
            "404 Not Found",
            &[("Content-Type", "text/plain")],
            b"missing",
        ),
    }) else {
        return;
    };

    let dir = tempfile::tempdir().unwrap();
    let manifest = dir.path().join("sources.json");
    let sources = vec![
        Source {
            title: "Open normalized".to_string(),
            url: format!("{server}/open"),
            access_status: " Open_Access ".to_string(),
            ..Source::default()
        },
        Source {
            title: "Unknown blocked".to_string(),
            url: format!("{server}/open"),
            access_status: " Unknown ".to_string(),
            ..Source::default()
        },
        Source {
            title: "Server failure".to_string(),
            url: format!("{server}/fail"),
            access_status: "free_web".to_string(),
            ..Source::default()
        },
    ];
    fs::write(&manifest, serde_json::to_string(&sources).unwrap()).unwrap();
    let out_dir = dir.path().join("downloads");
    run_download_sources(&[
        "--manifest".to_string(),
        manifest.display().to_string(),
        "--out-dir".to_string(),
        out_dir.display().to_string(),
        "--allow-status".to_string(),
        " OPEN_ACCESS , FREE_WEB ".to_string(),
        "--timeout".to_string(),
        "5".to_string(),
    ])
    .unwrap();

    let records = read_download_records(&out_dir.join("download-log.jsonl"));
    assert_eq!(records.len(), 3);
    let by_title = records
        .into_iter()
        .map(|record| (record.title.clone(), record))
        .collect::<HashMap<_, _>>();
    let open = &by_title["Open normalized"];
    assert_eq!(open.status, "downloaded");
    assert_eq!(open.bytes, "open body".len() as i64);
    assert_eq!(open.content_type, "text/plain");
    let base = Path::new(&open.path).file_name().unwrap().to_string_lossy();
    assert!(!base.contains(".."));
    assert!(base.ends_with("-Unsafe-Notes.txt"));

    let blocked = &by_title["Unknown blocked"];
    assert_eq!(blocked.status, "skipped");
    assert_eq!(blocked.reason, "blocked access status: unknown");

    let failed = &by_title["Server failure"];
    assert_eq!(failed.status, "error");
    assert_eq!(failed.reason, "HTTP 500");
}

#[test]
fn download_sources_removes_oversized_partial_file() {
    let body = vec![b'x'; 1024 * 1024 + 1];
    let Some(server) = spawn_server(1, move |_| {
        http_response(
            "200 OK",
            &[
                ("Content-Type", "application/pdf"),
                (
                    "Content-Disposition",
                    r#"attachment; filename="../too-big.pdf""#,
                ),
            ],
            &body,
        )
    }) else {
        return;
    };

    let dir = tempfile::tempdir().unwrap();
    let manifest = dir.path().join("sources.json");
    let sources = vec![Source {
        title: "Too big".to_string(),
        url: format!("{server}/too-big.pdf"),
        access_status: "free_web".to_string(),
        ..Source::default()
    }];
    fs::write(&manifest, serde_json::to_string(&sources).unwrap()).unwrap();
    let out_dir = dir.path().join("downloads");
    run_download_sources(&[
        "--manifest".to_string(),
        manifest.display().to_string(),
        "--out-dir".to_string(),
        out_dir.display().to_string(),
        "--max-mb".to_string(),
        "1".to_string(),
    ])
    .unwrap();

    let records = read_download_records(&out_dir.join("download-log.jsonl"));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status, "error");
    assert_contains(&records[0].reason, "download exceeded 1048576 bytes limit");
    let entries = fs::read_dir(&out_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert_eq!(entries, vec!["download-log.jsonl"]);
}
