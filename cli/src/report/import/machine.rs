use super::*;

#[derive(Debug, Default)]
pub(super) struct MachineInputPackages {
    pub(super) has_input: bool,
    knowledge: Vec<MachinePackage<KnowledgePackage>>,
    evidence: Vec<MachinePackage<EvidencePackage>>,
    pedagogy: Vec<MachinePackage<PedagogyPackage>>,
    pub(super) diagnostics: Vec<DiagnosticCheck>,
}

impl MachineInputPackages {
    fn has_packages(&self) -> bool {
        !self.knowledge.is_empty() || !self.evidence.is_empty() || !self.pedagogy.is_empty()
    }
}

#[derive(Debug)]
struct MachinePackage<T> {
    source_label: String,
    package: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MachinePackageKind {
    Knowledge,
    Evidence,
    Pedagogy,
}

impl MachinePackageKind {
    fn label(self) -> &'static str {
        match self {
            Self::Knowledge => "knowledge",
            Self::Evidence => "evidence",
            Self::Pedagogy => "pedagogy",
        }
    }

    fn expected_schema_version(self) -> &'static str {
        match self {
            Self::Knowledge => KNOWLEDGE_SCHEMA_VERSION,
            Self::Evidence => EVIDENCE_SCHEMA_VERSION,
            Self::Pedagogy => PEDAGOGY_SCHEMA_VERSION,
        }
    }

    fn target_path(self) -> &'static str {
        match self {
            Self::Knowledge => "/machine/knowledge",
            Self::Evidence => "/machine/evidence",
            Self::Pedagogy => "/machine/pedagogy",
        }
    }
}

pub(super) fn collect_machine_inputs(
    markdown: &str,
    paths: &MachineInputPaths,
) -> Result<MachineInputPackages> {
    let mut inputs = MachineInputPackages::default();
    if let Some(path) = paths.knowledge.as_deref() {
        inputs.has_input = true;
        read_machine_sidecar(path, MachinePackageKind::Knowledge, &mut inputs)?;
    }
    if let Some(path) = paths.evidence_package.as_deref() {
        inputs.has_input = true;
        read_machine_sidecar(path, MachinePackageKind::Evidence, &mut inputs)?;
    }
    if let Some(path) = paths.pedagogy.as_deref() {
        inputs.has_input = true;
        read_machine_sidecar(path, MachinePackageKind::Pedagogy, &mut inputs)?;
    }
    collect_sok_json_blocks(markdown, &mut inputs);
    Ok(inputs)
}

fn read_machine_sidecar(
    path: &Path,
    kind: MachinePackageKind,
    inputs: &mut MachineInputPackages,
) -> Result<()> {
    let source_label = path.display().to_string();
    let data = fs::read_to_string(path)
        .with_context(|| format!("read {} sidecar {}", kind.label(), path.display()))?;
    let value = match serde_json::from_str::<Value>(&data) {
        Ok(value) => value,
        Err(err) => {
            inputs.diagnostics.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_MACHINE_INPUT,
                    format!(
                        "{} sidecar {} is not valid JSON: {err}",
                        kind.label(),
                        path.display()
                    ),
                )
                .with_target(kind.target_path(), &source_label),
            );
            return Ok(());
        }
    };
    push_typed_machine_value(value, kind, source_label, inputs);
    Ok(())
}

fn collect_sok_json_blocks(markdown: &str, inputs: &mut MachineInputPackages) {
    let mut active: Option<(MarkdownFence, String, usize, Vec<String>)> = None;
    let mut ordinary_fence: Option<MarkdownFence> = None;
    for (line_index, line) in markdown.lines().enumerate() {
        if let Some((fence, info, start_line, body)) = active.as_mut() {
            if markdown_fence_closer(line, *fence) {
                let body_text = body.join("\n");
                process_sok_json_block(info, &body_text, *start_line, inputs);
                active = None;
            } else {
                body.push(line.to_string());
            }
            continue;
        }

        if let Some(fence) = ordinary_fence {
            if markdown_fence_closer(line, fence) {
                ordinary_fence = None;
            }
            continue;
        }

        let Some((fence, info)) = markdown_fence_opener(line) else {
            continue;
        };
        if is_sok_json_info(&info) {
            inputs.has_input = true;
            active = Some((fence, info, line_index + 1, Vec::new()));
        } else {
            ordinary_fence = Some(fence);
        }
    }

    if let Some((_, _, start_line, _)) = active {
        inputs.diagnostics.push(
            DiagnosticCheck::error(
                CHECK_EXPORT_MACHINE_INPUT,
                format!("sok-json block starting on line {start_line} is not closed"),
            )
            .with_target(format!("/machine/sok-json/line-{start_line}"), ""),
        );
    }
}

fn process_sok_json_block(
    info: &str,
    body: &str,
    start_line: usize,
    inputs: &mut MachineInputPackages,
) {
    let source_label = format!("sok-json block line {start_line}");
    let value = match serde_json::from_str::<Value>(body) {
        Ok(value) => value,
        Err(err) => {
            inputs.diagnostics.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_MACHINE_INPUT,
                    format!("sok-json block on line {start_line} is not valid JSON: {err}"),
                )
                .with_target(format!("/machine/sok-json/line-{start_line}"), ""),
            );
            return;
        }
    };
    let Some(schema_version) = value.get("schema_version").and_then(Value::as_str) else {
        inputs.diagnostics.push(
            DiagnosticCheck::error(
                CHECK_EXPORT_MACHINE_INPUT,
                format!("sok-json block on line {start_line} must declare schema_version"),
            )
            .with_target(format!("/machine/sok-json/line-{start_line}"), ""),
        );
        return;
    };
    if let Some(declared) = declared_sok_json_schema_version(info) {
        if declared != schema_version {
            inputs.diagnostics.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_MACHINE_INPUT,
                    format!(
                        "sok-json block on line {start_line} declares {declared:?} but JSON has \
                            schema_version {schema_version:?}"
                    ),
                )
                .with_target(format!("/machine/sok-json/line-{start_line}"), ""),
            );
            return;
        }
    }
    let Some(kind) = machine_kind_from_schema_version(schema_version) else {
        inputs.diagnostics.push(
            DiagnosticCheck::error(
                CHECK_EXPORT_MACHINE_INPUT,
                format!(
                    "sok-json block on line {start_line} has unsupported schema_version {schema_version:?}"
                ),
            )
            .with_target(format!("/machine/sok-json/line-{start_line}"), ""),
        );
        return;
    };
    push_typed_machine_value(value, kind, source_label, inputs);
}

fn push_typed_machine_value(
    value: Value,
    kind: MachinePackageKind,
    source_label: String,
    inputs: &mut MachineInputPackages,
) {
    match kind {
        MachinePackageKind::Knowledge => match serde_json::from_value::<KnowledgePackage>(value) {
            Ok(package) => inputs.knowledge.push(MachinePackage {
                source_label,
                package,
            }),
            Err(err) => {
                inputs
                    .diagnostics
                    .push(machine_deserialize_diagnostic(kind, &source_label, err))
            }
        },
        MachinePackageKind::Evidence => match serde_json::from_value::<EvidencePackage>(value) {
            Ok(package) => inputs.evidence.push(MachinePackage {
                source_label,
                package,
            }),
            Err(err) => {
                inputs
                    .diagnostics
                    .push(machine_deserialize_diagnostic(kind, &source_label, err))
            }
        },
        MachinePackageKind::Pedagogy => match serde_json::from_value::<PedagogyPackage>(value) {
            Ok(package) => inputs.pedagogy.push(MachinePackage {
                source_label,
                package,
            }),
            Err(err) => {
                inputs
                    .diagnostics
                    .push(machine_deserialize_diagnostic(kind, &source_label, err))
            }
        },
    }
}

fn machine_deserialize_diagnostic(
    kind: MachinePackageKind,
    source_label: &str,
    err: serde_json::Error,
) -> DiagnosticCheck {
    DiagnosticCheck::error(
        CHECK_EXPORT_MACHINE_INPUT,
        format!(
            "{} machine input {source_label} does not match {}: {err}",
            kind.label(),
            kind.expected_schema_version()
        ),
    )
    .with_target(kind.target_path(), source_label)
}

fn machine_kind_from_schema_version(schema_version: &str) -> Option<MachinePackageKind> {
    match schema_version {
        KNOWLEDGE_SCHEMA_VERSION => Some(MachinePackageKind::Knowledge),
        EVIDENCE_SCHEMA_VERSION => Some(MachinePackageKind::Evidence),
        PEDAGOGY_SCHEMA_VERSION => Some(MachinePackageKind::Pedagogy),
        _ => None,
    }
}

pub(super) fn project_report_with_machine_inputs(
    mut document: ReportDocument,
    machine_packages: MachineInputPackages,
    mut diagnostics: Vec<DiagnosticCheck>,
) -> Result<ReportDocument> {
    if !machine_packages.has_packages() {
        attach_export_diagnostics(&mut document, diagnostics);
        return crate::core::projection::project_report_document(&document);
    }

    let mut packages = crate::core::projection::core_packages_from_report(&document);
    merge_machine_packages(&mut packages, machine_packages, &mut diagnostics);
    reconcile_machine_satisfied_diagnostics(&packages, &mut diagnostics);
    let core_validation = crate::core::validation::validate_core_packages(&packages);
    let has_core_errors = core_validation.has_errors();
    diagnostics.extend(core_validation.diagnostics.checks);
    if has_core_errors {
        attach_export_diagnostics(&mut document, diagnostics);
        return Ok(document);
    }

    attach_export_diagnostics(&mut document, diagnostics.clone());
    let mut projected =
        crate::core::projection::project_report_compatibility(&packages, &document)?;
    attach_export_diagnostics(&mut projected, diagnostics);
    Ok(projected)
}

fn merge_machine_packages(
    packages: &mut CorePackages,
    mut machine_packages: MachineInputPackages,
    diagnostics: &mut Vec<DiagnosticCheck>,
) {
    for incoming in machine_packages.knowledge.drain(..) {
        merge_schema_version(
            &mut packages.knowledge.schema_version,
            &incoming.package.schema_version,
            MachinePackageKind::Knowledge,
            &incoming.source_label,
            diagnostics,
        );
        merge_string_field(
            &mut packages.knowledge.field,
            &incoming.package.field,
            "/knowledge/field",
            &incoming.source_label,
            diagnostics,
        );
        merge_by_id(
            &mut packages.knowledge.elements,
            incoming.package.elements,
            "/knowledge/elements",
            "knowledge element",
            &incoming.source_label,
            |element| element.id.as_str(),
            diagnostics,
        );
        merge_by_id(
            &mut packages.knowledge.relations,
            incoming.package.relations,
            "/knowledge/relations",
            "relation",
            &incoming.source_label,
            |relation| relation.id.as_str(),
            diagnostics,
        );
        packages.knowledge.relations = core::relations::deduplicate_relations(std::mem::take(
            &mut packages.knowledge.relations,
        ));
    }
    for incoming in machine_packages.evidence.drain(..) {
        merge_schema_version(
            &mut packages.evidence.schema_version,
            &incoming.package.schema_version,
            MachinePackageKind::Evidence,
            &incoming.source_label,
            diagnostics,
        );
        merge_by_id(
            &mut packages.evidence.sources,
            incoming.package.sources,
            "/evidence/sources",
            "evidence source",
            &incoming.source_label,
            |source| source.id.as_str(),
            diagnostics,
        );
        merge_by_id(
            &mut packages.evidence.claims,
            incoming.package.claims,
            "/evidence/claims",
            "evidence claim",
            &incoming.source_label,
            |claim| claim.id.as_str(),
            diagnostics,
        );
    }
    for incoming in machine_packages.pedagogy.drain(..) {
        merge_schema_version(
            &mut packages.pedagogy.schema_version,
            &incoming.package.schema_version,
            MachinePackageKind::Pedagogy,
            &incoming.source_label,
            diagnostics,
        );
        merge_by_id(
            &mut packages.pedagogy.reading_ladder,
            incoming.package.reading_ladder,
            "/pedagogy/reading_ladder",
            "reading ladder row",
            &incoming.source_label,
            |row| row.id.as_str(),
            diagnostics,
        );
        merge_by_id(
            &mut packages.pedagogy.learning_path,
            incoming.package.learning_path,
            "/pedagogy/learning_path",
            "learning step",
            &incoming.source_label,
            |step| step.id.as_str(),
            diagnostics,
        );
    }
}

fn reconcile_machine_satisfied_diagnostics(
    packages: &CorePackages,
    diagnostics: &mut Vec<DiagnosticCheck>,
) {
    if packages.knowledge.elements.is_empty() {
        return;
    }
    diagnostics.retain(|check| {
        !(check.check_id == CHECK_EXPORT_MISSING_PUBLIC_FIELD
            && check
                .message
                .contains("could not infer any field elements from structured Markdown surfaces"))
            && !(check.check_id == CHECK_EXPORT_REPORT_ARCHITECTURE
                && (check.target_path.starts_with("/report/field_elements")
                    || check.message.contains("field-element inventory")))
    });
}

fn merge_schema_version(
    target: &mut String,
    incoming: &str,
    kind: MachinePackageKind,
    source_label: &str,
    diagnostics: &mut Vec<DiagnosticCheck>,
) {
    if incoming == kind.expected_schema_version() {
        return;
    }
    if target == kind.expected_schema_version() {
        *target = incoming.to_string();
    }
    diagnostics.push(
        DiagnosticCheck::error(
            CHECK_EXPORT_MACHINE_INPUT,
            format!(
                "{} machine input {source_label} has unsupported schema_version {incoming:?}; expected {}",
                kind.label(),
                kind.expected_schema_version()
            ),
        )
        .with_target(format!("{}/schema_version", kind.target_path()), source_label),
    );
}

fn merge_string_field(
    target: &mut String,
    incoming: &str,
    target_path: &str,
    source_label: &str,
    diagnostics: &mut Vec<DiagnosticCheck>,
) {
    if incoming.trim().is_empty() || incoming == target {
        return;
    }
    if target.trim().is_empty() {
        *target = incoming.to_string();
        return;
    }
    diagnostics.push(
        DiagnosticCheck::error(
            CHECK_EXPORT_MACHINE_INPUT,
            format!(
                "machine input {source_label} conflicts with existing input lane at \
                    {target_path}; kept first value"
            ),
        )
        .with_target(target_path, source_label),
    );
}

fn merge_by_id<T, F>(
    target: &mut Vec<T>,
    incoming: Vec<T>,
    target_path: &str,
    item_label: &str,
    source_label: &str,
    id: F,
    diagnostics: &mut Vec<DiagnosticCheck>,
) where
    T: Clone + Serialize,
    F: Fn(&T) -> &str,
{
    for item in incoming {
        let item_id = id(&item).to_string();
        if let Some(existing) = target.iter().find(|candidate| id(candidate) == item_id) {
            if machine_values_equal(existing, &item) {
                continue;
            }
            diagnostics.push(
                DiagnosticCheck::error(
                    CHECK_EXPORT_MACHINE_INPUT,
                    format!(
                        "machine {item_label} {item_id} from {source_label} conflicts with \
                            existing input lane; kept first value"
                    ),
                )
                .with_target(target_path, &item_id),
            );
        } else {
            target.push(item);
        }
    }
}

fn machine_values_equal<T: Serialize>(left: &T, right: &T) -> bool {
    serde_json::to_value(left).ok() == serde_json::to_value(right).ok()
}
