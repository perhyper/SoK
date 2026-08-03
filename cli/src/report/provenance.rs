use super::ids::*;
use super::model::*;
use super::validation::*;
use super::*;

impl EvidenceEntry {
    pub fn cataloged_from_source(
        source: &Source,
        report_source: &ReportSource,
        input_path: &Path,
        row_number: usize,
    ) -> Self {
        let source_key = source_content_key(source);
        Self {
            evidence_id: content_id("ev", &[&report_source.id, &source_key, "cataloged"]),
            source_id: report_source.id.clone(),
            input_provenance: BoundedInputProvenance {
                input_path: input_path.display().to_string(),
                input_kind: input_kind_for_path(input_path),
                row_number: Some(row_number),
                row_hash: short_hash(&normalize_id_text(&source_key), 16),
            },
            verification_status: VerificationStatus::Cataloged,
            support_kind: SupportKind::Background,
            locator: String::new(),
            support_note: String::new(),
            notes: "Cataloged from source manifest only; this row has not been read, crawled, or externally verified and cannot satisfy claim evidence.".to_string(),
            claim_ids: Vec::new(),
            source_roles: report_source.roles.clone(),
            source_access: Some(report_source.access.clone()),
            source_citation: report_source.citation.clone(),
            source_identifier: report_source.identifier.clone(),
            source_url: report_source.url.clone(),
            curricular_use: source_curricular_use(source),
            ..Self::default()
        }
    }

    pub fn has_usable_support_metadata(&self) -> bool {
        !self.locator.trim().is_empty() || !self.support_note.trim().is_empty()
    }

    pub fn can_satisfy_claim_link(&self) -> bool {
        matches!(
            self.verification_status,
            VerificationStatus::Reviewed | VerificationStatus::Verified
        ) && self.support_kind == SupportKind::Supports
            && self.has_usable_support_metadata()
            && looks_like_iso_date(&self.reviewed_at)
    }

    pub fn to_evidence_link(&self) -> Option<EvidenceLink> {
        if !self.can_satisfy_claim_link() {
            return None;
        }
        Some(self.as_evidence_link())
    }

    pub fn to_visible_evidence_link(&self) -> Option<EvidenceLink> {
        match self.verification_status {
            VerificationStatus::Cataloged => Some(self.as_evidence_link()),
            VerificationStatus::Reviewed | VerificationStatus::Verified
                if self.has_usable_support_metadata() && looks_like_iso_date(&self.reviewed_at) =>
            {
                Some(self.as_evidence_link())
            }
            _ => None,
        }
    }

    fn as_evidence_link(&self) -> EvidenceLink {
        EvidenceLink {
            evidence_id: self.evidence_id.clone(),
            source_id: self.source_id.clone(),
            verification_status: self.verification_status,
            support_kind: self.support_kind,
            locator: self.locator.clone(),
            support_note: self.support_note.clone(),
            reviewed_at: self.reviewed_at.clone(),
        }
    }
}

impl AccessStatus {
    pub fn from_manifest_status(raw: &str) -> Self {
        match normalize_access_status_label(raw).as_str() {
            "open" | "open_access" | "oa" | "cc_by" | "cc_by_sa" => Self::OpenAccess,
            "free" | "free_web" | "free_to_read" => Self::FreeWeb,
            "public_domain" => Self::PublicDomain,
            "official" | "official_open" => Self::OfficialOpen,
            "user" | "user_provided" => Self::UserProvided,
            "library" | "library_access" => Self::Library,
            "paid" | "paid_book" | "book_purchase" => Self::PaidBook,
            "paywall" | "paywalled" => Self::Paywalled,
            "subscription" | "institutional_subscription" => Self::Subscription,
            "restricted" => Self::Restricted,
            _ => Self::Unknown,
        }
    }

    pub fn metadata_only_default(self) -> bool {
        matches!(
            self,
            Self::PaidBook
                | Self::Paywalled
                | Self::Subscription
                | Self::Restricted
                | Self::Unknown
        )
    }
}

pub fn normalize_source_manifest<P: AsRef<Path>>(path: P) -> Result<NormalizedSourceManifest> {
    let path = path.as_ref();
    let raw_sources = load_sources(path)?;
    let sources = normalize_report_sources(&raw_sources);
    let evidence = raw_sources
        .iter()
        .zip(sources.iter())
        .enumerate()
        .map(|(index, (source, report_source))| {
            EvidenceEntry::cataloged_from_source(source, report_source, path, index + 1)
        })
        .collect::<Vec<_>>();
    let diagnostics = source_manifest_diagnostics(&sources);
    Ok(NormalizedSourceManifest {
        sources,
        evidence,
        diagnostics,
    })
}

pub fn normalize_report_sources(sources: &[Source]) -> Vec<ReportSource> {
    let keys = sources.iter().map(source_content_key).collect::<Vec<_>>();
    let id_map = content_id_map("src", keys.iter().map(String::as_str));
    sources
        .iter()
        .zip(keys.iter())
        .map(|(source, key)| {
            let normalized = normalize_id_text(key);
            let id = id_map
                .get(&normalized)
                .cloned()
                .unwrap_or_else(|| content_id("src", &[key]));
            normalize_report_source_with_id(source, id)
        })
        .collect()
}

pub fn normalize_report_source(source: &Source) -> ReportSource {
    normalize_report_source_with_id(source, source_report_id(source))
}

pub fn source_report_id(source: &Source) -> String {
    content_id("src", &[&source_content_key(source)])
}

pub fn normalize_report_source_with_id(source: &Source, id: String) -> ReportSource {
    let access = SourceAccessMetadata::from_source(source);
    let citation = first_non_empty([
        source.citation.as_str(),
        source.title.as_str(),
        source.identifier.as_str(),
        source.url.as_str(),
        "Untitled source",
    ]);
    let why_it_matters = first_non_empty([
        source.why_it_matters.as_str(),
        source.use_in_curriculum.as_str(),
        source.notes.as_str(),
        "Cataloged source; source role or relevance not yet specified.",
    ]);
    ReportSource {
        id,
        citation,
        title: source.title.trim().to_string(),
        source_type: first_non_empty([source.source_type.as_str(), "unknown"]),
        identifier: source.identifier.trim().to_string(),
        url: source.url.trim().to_string(),
        date: normalize_report_date(&source.date),
        roles: normalize_source_roles(&source.layer, &source.source_type),
        access,
        why_it_matters,
        verification_status: VerificationStatus::Cataloged,
        notes: source.notes.trim().to_string(),
        ..ReportSource::default()
    }
}

impl SourceAccessMetadata {
    pub fn from_source(source: &Source) -> Self {
        let access = source_access(source);
        let status = AccessStatus::from_manifest_status(&access.status);
        let route = first_non_empty([
            access.route.as_str(),
            source.access_route.as_str(),
            source.url.as_str(),
            "unknown",
        ]);
        Self {
            status,
            route,
            budget_estimate: first_non_empty([
                access.budget_estimate.as_str(),
                source.budget_estimate.as_str(),
            ]),
            license: source.license.trim().to_string(),
            metadata_only: Some(status.metadata_only_default()),
            notes: first_non_empty([access.notes.as_str(), source.notes.as_str()]),
        }
    }
}

pub fn normalize_source_roles(layer: &str, source_type: &str) -> Vec<SourceRole> {
    let haystack = format!(
        " {} {} ",
        normalize_access_status_label(layer),
        normalize_access_status_label(source_type)
    );
    let mut roles = BTreeSet::new();
    for (needle, role) in [
        ("orientation", SourceRole::Orientation),
        ("foundation", SourceRole::Foundation),
        ("method", SourceRole::Method),
        ("representation", SourceRole::Representation),
        ("evidence", SourceRole::Evidence),
        ("synthesis", SourceRole::Synthesis),
        ("survey", SourceRole::Synthesis),
        ("frontier", SourceRole::Frontier),
        ("debate", SourceRole::Debate),
        ("standard", SourceRole::Standard),
        ("dataset", SourceRole::Dataset),
        ("data", SourceRole::Dataset),
        ("infrastructure", SourceRole::Infrastructure),
        ("critique", SourceRole::Critique),
        ("curriculum", SourceRole::Curriculum),
        ("syllabus", SourceRole::Curriculum),
    ] {
        if haystack.contains(needle) {
            roles.insert(role);
        }
    }
    if roles.is_empty() {
        roles.insert(SourceRole::Other);
    }
    roles.into_iter().collect()
}

pub fn source_curricular_use(source: &Source) -> String {
    first_non_empty([
        source.use_in_curriculum.as_str(),
        source.why_it_matters.as_str(),
        source.layer.as_str(),
    ])
}

pub fn normalize_report_date(raw: &str) -> String {
    let value = raw.trim();
    let lowered = value.to_ascii_lowercase();
    if value.is_empty()
        || matches!(
            lowered.as_str(),
            "unknown" | "n/a" | "na" | "date after lookup" | "to verify"
        )
    {
        String::new()
    } else {
        value.to_string()
    }
}

pub(crate) fn review_after_date(as_of: &str) -> String {
    NaiveDate::parse_from_str(as_of, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.checked_add_signed(Duration::days(183)))
        .map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

pub fn source_manifest_diagnostics(sources: &[ReportSource]) -> Vec<DiagnosticCheck> {
    let mut diagnostics = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let target_path = format!("/sources/{index}");
        if source.access.status == AccessStatus::Unknown {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_SOURCE_MISSING_ACCESS_STATUS,
                    format!("source {} has unknown access status", source.id),
                )
                .with_target(&target_path, &source.id),
            );
        }
        if source.access.route.trim().is_empty() || source.access.route == "unknown" {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_SOURCE_MISSING_ACCESS_ROUTE,
                    format!("source {} has no actionable access route", source.id),
                )
                .with_target(&target_path, &source.id),
            );
        }
        if source
            .why_it_matters
            .starts_with("Cataloged source; source role or relevance")
        {
            diagnostics.push(
                DiagnosticCheck::warning(
                    CHECK_SOURCE_MISSING_CURRICULAR_USE,
                    format!("source {} has no curricular use note", source.id),
                )
                .with_target(&target_path, &source.id),
            );
        }
    }
    diagnostics
}

pub(crate) fn source_content_key(source: &Source) -> String {
    [
        source.citation.trim(),
        source.title.trim(),
        source.identifier.trim(),
        source.url.trim(),
        source.source_type.trim(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

pub(crate) fn first_non_empty<const N: usize>(values: [&str; N]) -> String {
    values
        .into_iter()
        .find(|value| !value.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string()
}

pub(crate) fn normalize_access_status_label(raw: &str) -> String {
    normalize_id_text(raw).replace(' ', "_")
}

pub(crate) fn input_kind_for_path(path: &Path) -> String {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "csv" => "source_manifest_csv",
        "tsv" => "source_manifest_tsv",
        "json" => "source_manifest_json",
        _ => "source_manifest",
    }
    .to_string()
}
