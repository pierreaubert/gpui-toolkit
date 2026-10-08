use clap::{CommandFactory, Parser};
use gpui_toolkit_cli::{
    API_VERSION, CATALOG_BEGIN, CATALOG_END, COMMANDS, Cli, DetailLevel, ErrorCode, Suggestion,
    ToolkitError, build_manifest, command_static, component_detail, component_list,
    component_props, eject_component_from_root, error_envelope, find_toolkit_root,
    parse_detail_level, render_json, run_command, run_init, search, success_envelope,
    template_copy, template_list, template_show, template_skeleton,
};
use std::collections::HashSet;

#[test]
fn error_codes_are_stable() {
    assert_eq!(ErrorCode::Unknown.as_str(), "ERR_UNKNOWN");
    assert_eq!(ErrorCode::InvalidArgument.as_str(), "ERR_INVALID_ARGUMENT");
    assert_eq!(ErrorCode::WriteFailed.as_str(), "ERR_WRITE_FAILED");
    assert_eq!(ErrorCode::CatalogLoad.as_str(), "ERR_CATALOG_LOAD");
    assert_eq!(
        ErrorCode::UnknownComponent.as_str(),
        "ERR_UNKNOWN_COMPONENT"
    );
    assert_eq!(ErrorCode::UnknownTemplate.as_str(), "ERR_UNKNOWN_TEMPLATE");
    assert_eq!(ErrorCode::FileExists.as_str(), "ERR_FILE_EXISTS");
    assert_eq!(ErrorCode::NoSource.as_str(), "ERR_NO_SOURCE");
    assert_eq!(ErrorCode::UnknownTheme.as_str(), "ERR_UNKNOWN_THEME");
    assert_eq!(ErrorCode::LayoutParse.as_str(), "ERR_LAYOUT_PARSE");
    assert_eq!(ErrorCode::LayoutInvalid.as_str(), "ERR_LAYOUT_INVALID");
    assert_eq!(ErrorCode::InvalidConfig.as_str(), "ERR_INVALID_CONFIG");
}

#[test]
fn error_envelope_shape() {
    let error = ToolkitError::new(ErrorCode::InvalidArgument, "bad dir")
        .with_suggestions([Suggestion::new("init", "similar name")]);
    let text = render_json(&error_envelope(&error)).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["apiVersion"], API_VERSION);
    assert_eq!(value["error"], "bad dir");
    assert_eq!(value["code"], "ERR_INVALID_ARGUMENT");
    assert_eq!(value["suggestions"][0]["name"], "init");

    let bare = ToolkitError::new(ErrorCode::Unknown, "nope");
    let text = render_json(&error_envelope(&bare)).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(value.get("suggestions").is_none());
}

#[test]
fn manifest_json_shape() {
    let command = Cli::command();
    let manifest = build_manifest(&command);
    let text = render_json(&success_envelope("manifest", &manifest)).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["apiVersion"], API_VERSION);
    assert_eq!(value["type"], "manifest");
    let names: HashSet<&str> = value["data"]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert!(names.contains("manifest"));
    assert!(names.contains("init"));
    let supported: HashSet<&str> = value["data"]["jsonSupported"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry.as_str().unwrap())
        .collect();
    assert!(names.contains("theme"));
    assert!(supported.contains("theme build"));
    assert!(supported.contains("theme list"));
    assert!(supported.contains("theme targets"));
    assert!(!supported.contains("theme"));
    assert!(
        value["data"]["globalOptions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|option| option["flag"] == "--json" && option["type"] == "boolean")
    );
    let codes: Vec<i64> = value["data"]["exitCodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["code"].as_i64().unwrap())
        .collect();
    assert_eq!(codes, vec![0, 1, 2]);
    let component = value["data"]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "component")
        .unwrap();
    let responses: Vec<&str> = component["responseTypes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry.as_str().unwrap())
        .collect();
    assert!(responses.contains(&"component.batch"));
    assert!(responses.contains(&"error"));
}

fn collect_paths(command: &clap::Command, prefix: &str, paths: &mut Vec<String>) {
    for subcommand in command.get_subcommands() {
        let path = if prefix.is_empty() {
            subcommand.get_name().to_owned()
        } else {
            format!("{prefix} {}", subcommand.get_name())
        };
        paths.push(path.clone());
        collect_paths(subcommand, &path, paths);
    }
}

#[test]
fn drift_manifest_covers_subcommands() {
    let command = Cli::command();
    let mut clap_paths = Vec::new();
    collect_paths(&command, "", &mut clap_paths);
    assert!(!clap_paths.is_empty());

    for path in &clap_paths {
        let facts = command_static(path);
        assert!(!facts.examples.is_empty(), "missing examples for {path}");
    }
    let known: HashSet<&str> = clap_paths.iter().map(String::as_str).collect();
    for facts in COMMANDS {
        assert!(
            known.contains(facts.name),
            "stale manifest entry '{}'",
            facts.name
        );
        assert!(!facts.examples.is_empty());
        if facts.json {
            assert!(
                !facts.responses.is_empty(),
                "missing response types for {}",
                facts.name
            );
            assert!(
                facts.responses.contains(&"error"),
                "missing error response for {}",
                facts.name
            );
        } else {
            assert!(
                facts.responses.is_empty(),
                "bare group '{}' must not list responses",
                facts.name
            );
        }
    }

    let manifest = build_manifest(&command);
    let mut entries = manifest.commands.clone();
    while let Some(entry) = entries.pop() {
        assert!(
            !entry.description.is_empty(),
            "missing help for {}",
            entry.name
        );
        assert!(!entry.examples.is_empty());
        entries.extend(entry.subcommands.clone());
    }
    let supported: HashSet<&str> = manifest.json_supported.iter().map(String::as_str).collect();
    for facts in COMMANDS.iter().filter(|entry| entry.json) {
        assert!(
            supported.contains(facts.name),
            "missing json flag for {}",
            facts.name
        );
    }
}

#[test]
fn init_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let options = gpui_toolkit_cli::InitOptions {
        dir: dir.path().to_path_buf(),
        agents_file: "AGENTS.md".to_owned(),
    };
    let receipt = run_init(&options).unwrap();
    assert!(receipt.created);
    assert!(!receipt.updated);
    assert!(receipt.stories > 0);
    let content = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(content.contains(CATALOG_BEGIN));
    assert!(content.contains(CATALOG_END));
    assert!(content.contains("ui-kit.button"));
}

#[test]
fn init_appends_then_refreshes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("AGENTS.md"), "# Notes\n").unwrap();
    let options = gpui_toolkit_cli::InitOptions {
        dir: dir.path().to_path_buf(),
        agents_file: "AGENTS.md".to_owned(),
    };
    let first = run_init(&options).unwrap();
    assert!(!first.created);
    assert!(!first.updated);
    let content = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(content.starts_with("# Notes\n"));
    assert!(content.contains(CATALOG_BEGIN));

    let second = run_init(&options).unwrap();
    assert!(second.updated);
    let refreshed = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert_eq!(content, refreshed);
    assert_eq!(refreshed.matches(CATALOG_BEGIN).count(), 1);
}

#[test]
fn init_rejects_traversal() {
    let dir = tempfile::tempdir().unwrap();
    for agents_file in ["../escape.md", "sub/dir.md", "..", ""] {
        let options = gpui_toolkit_cli::InitOptions {
            dir: dir.path().to_path_buf(),
            agents_file: agents_file.to_owned(),
        };
        let error = run_init(&options).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidArgument, "{agents_file}");
    }
}

#[test]
fn component_detail_resolves_id_and_title() {
    let by_id = component_detail("ui-kit.button").unwrap();
    assert_eq!(by_id.story.id, "ui-kit.button");
    assert!(by_id.follow_up.contains("ui-kit.button"));

    let by_title = component_detail("button").unwrap();
    assert_eq!(by_title.story.id, "ui-kit.button");
}

#[test]
fn component_detail_unknown_suggests() {
    let error = component_detail("butto").unwrap_err();
    assert_eq!(error.code, ErrorCode::UnknownComponent);
    assert!(!error.suggestions.is_empty());
    assert!(
        error
            .suggestions
            .iter()
            .any(|hint| hint.name == "ui-kit.button")
    );
}

#[test]
fn component_list_levels() {
    let brief = component_list(DetailLevel::Brief).unwrap();
    assert_eq!(brief.detail, "brief");
    assert_eq!(brief.count, brief.entries.len());
    assert!(brief.entries.iter().all(|entry| entry.title.is_none()));

    let compact = component_list(DetailLevel::Compact).unwrap();
    assert!(
        compact
            .entries
            .iter()
            .all(|entry| entry.title.is_some() && entry.description.is_none())
    );

    let full = component_list(DetailLevel::Full).unwrap();
    assert!(full.entries.iter().all(|entry| entry.description.is_some()));
    let ids: Vec<&str> = full.entries.iter().map(|entry| entry.id.as_str()).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted);
}

#[test]
fn component_props_only() {
    let props = component_props("ui-kit.button").unwrap();
    assert_eq!(props.id, "ui-kit.button");
    assert!(props.props.iter().any(|prop| prop.name == "label"));
}

#[test]
fn component_batch_mixes_hits_and_misses() {
    use gpui_toolkit_cli::{COMPONENT_BATCH_TYPE, component_batch, render_batch_text};

    assert_eq!(COMPONENT_BATCH_TYPE, "component.batch");
    let batch = component_batch(
        &[
            "ui-kit.button".to_owned(),
            "nope".to_owned(),
            "button".to_owned(),
        ],
        false,
    )
    .unwrap();
    assert_eq!(batch.mode, "detail");
    assert_eq!(batch.count, 3);
    assert_eq!(batch.ok, 2);
    assert_eq!(batch.errors, 1);
    assert_eq!(batch.results[0].status, "ok");
    assert!(batch.results[0].detail.is_some());
    assert_eq!(batch.results[1].status, "error");
    assert_eq!(batch.results[1].code, Some(ErrorCode::UnknownComponent));
    assert_eq!(batch.results[2].status, "ok");

    let props = component_batch(&["ui-kit.button".to_owned()], true).unwrap();
    assert_eq!(props.mode, "props");
    assert!(props.results[0].props.is_some());
    assert!(props.results[0].detail.is_none());

    let text = render_batch_text(&batch, false);
    assert!(text.contains("2 of 3 resolved"));
    let dense = render_batch_text(&batch, true);
    assert!(dense.contains("ok=2 errors=1"));
}

#[test]
fn component_command_batches_several_names() {
    let cli = Cli::try_parse_from([
        "gpui-toolkit",
        "component",
        "ui-kit.button",
        "nope",
        "--json",
    ])
    .unwrap();
    let output = run_command(&cli).unwrap();
    assert_eq!(output.exit_code, 0);
    let value: serde_json::Value = serde_json::from_str(&output.json).unwrap();
    assert_eq!(value["type"], "component.batch");
    assert_eq!(value["data"]["ok"], 1);
    assert_eq!(value["data"]["errors"], 1);

    let cli = Cli::try_parse_from(["gpui-toolkit", "component", "--props"]).unwrap();
    let error = run_command(&cli).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgument);
}

#[test]
fn search_ranks_and_limits() {
    let results = search("ui-kit.button", 20).unwrap();
    assert_eq!(results.results[0].id, "ui-kit.button");
    assert_eq!(results.results[0].score, 100);

    let limited = search("a", 3).unwrap();
    assert_eq!(limited.results.len(), 3);
    assert!(limited.match_count >= 3);
    let scores: Vec<u32> = limited.results.iter().map(|hit| hit.score).collect();
    let mut ordered = scores.clone();
    ordered.sort_unstable_by(|left, right| right.cmp(left));
    assert_eq!(scores, ordered);
}

#[test]
fn detail_parses_known_levels() {
    assert_eq!(parse_detail_level("brief").unwrap(), DetailLevel::Brief);
    assert_eq!(parse_detail_level("compact").unwrap(), DetailLevel::Compact);
    assert_eq!(parse_detail_level("full").unwrap(), DetailLevel::Full);
    let error = parse_detail_level("verbose").unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgument);
}

#[test]
fn templates_show_and_copy() {
    let list = template_list(false);
    assert_eq!(list.count, 4);
    assert!(list.templates.iter().all(|entry| !entry.hidden));
    for entry in &list.templates {
        let shown = template_show(&entry.id).unwrap();
        match shown.kind.as_str() {
            "page" | "block" => assert!(shown.source.contains("impl IntoElement")),
            "theme" => assert!(shown.source.contains("EditorTheme")),
            kind => unreachable!("unexpected template kind '{kind}'"),
        }
        let skeleton = template_skeleton(&entry.id).unwrap();
        assert!(!skeleton.skeleton.is_empty());
        assert_ne!(skeleton.skeleton, shown.source);
    }
    let all = template_list(true);
    assert_eq!(all.count, 5);
    assert!(all.templates.iter().any(|entry| entry.hidden));
    let hidden = template_show("empty-page").unwrap();
    assert!(hidden.source.contains("impl IntoElement"));
    let error = template_show("nope").unwrap_err();
    assert_eq!(error.code, ErrorCode::UnknownTemplate);

    let dir = tempfile::tempdir().unwrap();
    let copy = template_copy("settings-page", dir.path(), None).unwrap();
    assert!(copy.path.ends_with("settings_page.rs"));
    let content = std::fs::read_to_string(&copy.path).unwrap();
    assert!(content.contains("settings_page"));
    let error = template_copy("settings-page", dir.path(), None).unwrap_err();
    assert_eq!(error.code, ErrorCode::FileExists);
    let error = template_copy("settings-page", dir.path(), Some("../x.rs")).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgument);
}

#[test]
fn eject_rewrites_and_copies_companions() {
    let root = tempfile::tempdir().unwrap();
    let src = root.path().join("crates/gpui-ui-kit/src");
    std::fs::create_dir_all(src.join("widget")).unwrap();
    std::fs::write(
        src.join("widget.rs"),
        "use crate::theme::Theme;\nmod extra;\n",
    )
    .unwrap();
    std::fs::write(src.join("widget/extra.rs"), "use crate::Button;\n").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let receipt = eject_component_from_root("Widget", dir.path(), root.path()).unwrap();
    assert_eq!(receipt.component, "Widget");
    assert_eq!(receipt.files.len(), 2);
    assert!(receipt.issues_url.ends_with("/issues"));
    let main = std::fs::read_to_string(dir.path().join("widget.rs")).unwrap();
    assert!(main.contains("use gpui_ui_kit::theme::Theme;"));
    let companion = std::fs::read_to_string(dir.path().join("widget/extra.rs")).unwrap();
    assert!(companion.contains("use gpui_ui_kit::Button;"));

    let error = eject_component_from_root("Missing", dir.path(), root.path()).unwrap_err();
    assert_eq!(error.code, ErrorCode::NoSource);
    let error = eject_component_from_root("../Widget", dir.path(), root.path()).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgument);
}

#[test]
fn eject_snake_cases_and_aliases() {
    let root = tempfile::tempdir().unwrap();
    let src = root.path().join("crates/gpui-ui-kit/src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("number_input.rs"), "// ni\n").unwrap();
    std::fs::write(src.join("qr.rs"), "// qr\n").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let receipt = eject_component_from_root("NumberInput", dir.path(), root.path()).unwrap();
    assert_eq!(receipt.files, vec!["number_input.rs".to_owned()]);
    let receipt = eject_component_from_root("QrCode", dir.path(), root.path()).unwrap();
    assert_eq!(receipt.files, vec!["qr.rs".to_owned()]);
}

#[test]
fn toolkit_root_found_from_checkout() {
    let root = find_toolkit_root().expect("tests run inside the checkout");
    assert!(root.join("crates/gpui-ui-kit").is_dir());
}

#[test]
fn theme_build_check_and_targets() {
    use gpui_toolkit_cli::{theme_build, theme_check, theme_list, theme_targets};

    let list = theme_list();
    assert_eq!(list.count, 12);
    assert!(list.themes.iter().any(|entry| entry.id == "dark"));

    let dir = tempfile::tempdir().unwrap();
    let build = theme_build("dark", dir.path(), None).unwrap();
    assert_eq!(build.preset, "dark");
    assert!(build.token_count > 0);
    assert!(build.bytes > 0);
    assert!(build.path.ends_with("dark.tokens.json"));
    let content = std::fs::read_to_string(&build.path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(value["color"].as_array().unwrap().len(), build.token_count);

    let check = theme_check("dark", dir.path(), None).unwrap();
    assert!(check.up_to_date);
    std::fs::write(&build.path, "stale").unwrap();
    let check = theme_check("dark", dir.path(), None).unwrap();
    assert!(!check.up_to_date);
    assert_eq!(check.stale[0].reason, "outdated");
    std::fs::remove_file(&build.path).unwrap();
    let check = theme_check("dark", dir.path(), None).unwrap();
    assert_eq!(check.stale[0].reason, "missing");

    let targets = theme_targets("dark").unwrap();
    assert!(
        targets
            .targets
            .iter()
            .any(|target| target.key == "color.accent")
    );
    let keys: Vec<&str> = targets
        .targets
        .iter()
        .map(|target| target.key.as_str())
        .collect();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(keys, sorted);

    let error = theme_build("nope", dir.path(), None).unwrap_err();
    assert_eq!(error.code, ErrorCode::UnknownTheme);
}

#[test]
fn doctor_reports_and_codes() {
    use gpui_toolkit_cli::{
        DoctorCheck, DoctorReport, DoctorStatus, DoctorSummary, render_doctor_text, run_doctor,
        upgrade_list,
    };

    let failing = DoctorReport {
        checks: vec![DoctorCheck {
            id: String::from("catalog"),
            status: DoctorStatus::Fail,
            label: String::from("Component catalog"),
            message: String::from("seeded fault"),
            fix: None,
        }],
        summary: DoctorSummary {
            pass: 0,
            warn: 0,
            fail: 1,
            info: 0,
        },
    };
    assert!(failing.failed());
    assert!(render_doctor_text(&failing, false).contains("Failures need attention"));

    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(dir.path());
    assert_eq!(report.checks.len(), 5);
    let ids: Vec<&str> = report
        .checks
        .iter()
        .map(|check| check.id.as_str())
        .collect();
    assert!(ids.contains(&"config"));
    assert!(ids.contains(&"catalog"));
    assert!(ids.contains(&"story-coverage"));
    assert!(ids.contains(&"agent-docs"));
    assert!(ids.contains(&"checkout-root"));
    let total =
        report.summary.pass + report.summary.warn + report.summary.fail + report.summary.info;
    assert_eq!(total, 5);
    assert!(!report.failed());

    let cli = Cli::try_parse_from([
        "gpui-toolkit",
        "doctor",
        "--dir",
        dir.path().to_str().unwrap(),
    ])
    .unwrap();
    let output = run_command(&cli).unwrap();
    assert_eq!(output.exit_code, 0);
    assert!(output.json.contains("\"type\": \"doctor\""));

    let list = upgrade_list();
    assert!(!list.toolkit_version.is_empty());
    assert_eq!(list.count, 0);
}

#[test]
fn gap_report_validates_area() {
    use gpui_toolkit_cli::{GAP_AREAS, gap_report};

    assert!(GAP_AREAS.contains(&"component"));
    let report = gap_report("component", "Add DateRangePicker", None).unwrap();
    assert_eq!(report.area, "component");
    assert!(report.body.contains("Add DateRangePicker"));
    assert!(report.issues_url.ends_with("/issues"));
    let custom = gap_report("cli", "Title", Some("https://x.test/issues")).unwrap();
    assert_eq!(custom.issues_url, "https://x.test/issues");
    let error = gap_report("nope", "Title", None).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgument);
    let error = gap_report("cli", "   ", None).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgument);
}

#[test]
fn layout_check_expand_and_errors() {
    use gpui_layout_expr::CustomComponents;
    use gpui_toolkit_cli::{layout_check, layout_expand};

    let builtin = CustomComponents::empty();
    let checked = layout_check("V>(Tx\"Hi\"+B.primary\"Save\"#save)", &builtin).unwrap();
    assert_eq!(checked.canonical, "V > (Tx\"Hi\" + B#save.primary\"Save\")");
    assert_eq!(checked.nodes, 3);
    assert!(checked.custom_components.is_empty());

    let unit = layout_expand("V > B\"Hi\"", None, &builtin).unwrap();
    assert!(unit.contains("pub fn expanded_layout()"));
    assert!(unit.contains("VStack::new()"));
    assert!(unit.contains("Button::new(\"layout-0\", \"Hi\")"));

    let named = layout_expand("B\"Hi\"", Some("hero"), &builtin).unwrap();
    assert!(named.contains("pub fn hero()"));

    let error = layout_check("V > *", &builtin).unwrap_err();
    assert_eq!(error.code, ErrorCode::LayoutParse);
    assert!(error.message.contains("column 5"));
    let error = layout_check("V > X\"?\"", &builtin).unwrap_err();
    assert_eq!(error.code, ErrorCode::LayoutInvalid);
    let error = layout_expand("B\"a\" + B\"b\"", None, &builtin).unwrap_err();
    assert_eq!(error.code, ErrorCode::LayoutInvalid);
    let error = layout_expand("B\"Hi\"", Some("fn"), &builtin).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgument);
}

#[test]
fn layout_expand_to_file_refuses_overwrite() {
    use gpui_layout_expr::CustomComponents;
    use gpui_toolkit_cli::layout_expand_to_file;

    let builtin = CustomComponents::empty();
    let dir = tempfile::tempdir().unwrap();
    let first = layout_expand_to_file("B\"Hi\"", None, dir.path(), None, &builtin).unwrap();
    assert!(first.path.as_ref().unwrap().ends_with("expanded_layout.rs"));
    let error = layout_expand_to_file("B\"Hi\"", None, dir.path(), None, &builtin).unwrap_err();
    assert_eq!(error.code, ErrorCode::FileExists);
}

#[test]
fn layout_expand_matches_fixture() {
    use gpui_layout_expr::CustomComponents;
    use gpui_toolkit_cli::layout_expand;

    let expr = "V > (Tx\"Dashboard\" + (H > I#name[label=\"Name\"] + B.primary\"Go\"#go))";
    let unit = layout_expand(expr, Some("fixture_dashboard"), &CustomComponents::empty()).unwrap();
    let fixture = include_str!("fixtures/expanded_fixture_dashboard.rs").replace("\r\n", "\n");
    assert_eq!(unit, fixture);
}

#[test]
fn templates_carry_valid_layout_headers() {
    use gpui_layout_expr::CustomComponents;
    use gpui_toolkit_cli::{TEMPLATES, layout_check, template_show};

    let builtin = CustomComponents::empty();
    for entry in TEMPLATES {
        let shown = template_show(entry.id).unwrap();
        if shown.kind == "theme" {
            continue;
        }
        let header = shown.source.lines().next().unwrap();
        let expr = header
            .strip_prefix("// LAYOUT (")
            .and_then(|rest| rest.strip_suffix(')'))
            .unwrap_or_else(|| panic!("{}: malformed LAYOUT header", entry.id));
        let checked = layout_check(expr, &builtin)
            .unwrap_or_else(|error| panic!("{}: invalid LAYOUT header: {error}", entry.id));
        assert!(checked.nodes > 1, "{}: header is trivial", entry.id);
    }
}

#[test]
fn dense_json_is_single_line() {
    let cli = Cli::try_parse_from(["gpui-toolkit", "--dense", "search", "button"]).unwrap();
    let output = run_command(&cli).unwrap();
    assert!(!output.json.contains('\n'));
    assert!(!output.text.contains("\n\n"));
}

#[test]
fn init_rejects_missing_dir() {
    let dir = tempfile::tempdir().unwrap();
    let options = gpui_toolkit_cli::InitOptions {
        dir: dir.path().join("nope"),
        agents_file: "AGENTS.md".to_owned(),
    };
    let error = run_init(&options).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgument);
}

#[test]
fn config_discovers_upward_and_rejects_unknown_fields() {
    use gpui_toolkit_cli::{discover_config, parse_config, read_config_file};

    assert!(parse_config("").unwrap().layout.components.is_empty());
    parse_config("[project]\nname = 1\n").unwrap_err();
    parse_config("[layout.components]\n").unwrap_err();

    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a").join("b");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(
        dir.path().join("toolkit.toml"),
        "[[layout.components]]\nname = \"PrimaryButton\"\nbase = \"B\"\n",
    )
    .unwrap();
    let found = discover_config(&nested).unwrap();
    assert!(found.ends_with("toolkit.toml"));
    let loaded = read_config_file(&found).unwrap();
    assert_eq!(loaded.customs().unwrap().len(), 1);
}

#[test]
fn layout_check_reports_custom_components() {
    use gpui_layout_expr::CustomComponents;
    use gpui_toolkit_cli::{LoadedConfig, layout_check, parse_config};

    let config = parse_config(
        "[[layout.components]]\nname = \"PrimaryButton\"\nbase = \"B\"\nmodifier = \"primary\"\n",
    )
    .unwrap();
    let loaded = LoadedConfig { path: None, config };
    let customs = loaded.customs().unwrap();
    let checked = layout_check("V > PrimaryButton\"Save\"", &customs).unwrap();
    assert_eq!(checked.canonical, "V > PrimaryButton\"Save\"");
    assert_eq!(checked.custom_components, vec!["PrimaryButton"]);

    let builtin = CustomComponents::empty();
    let error = layout_check("V > PrimaryButton\"Save\"", &builtin).unwrap_err();
    assert_eq!(error.code, ErrorCode::LayoutInvalid);

    let error = layout_check("V > PrimaryButton.secondary\"Save\"", &customs).unwrap_err();
    assert_eq!(error.code, ErrorCode::LayoutInvalid);
    assert!(error.message.contains("twice"));
}

#[test]
fn layout_expand_resolves_custom_aliases() {
    use gpui_toolkit_cli::{LoadedConfig, layout_expand, parse_config};

    let config = parse_config(
        "[[layout.components]]\nname = \"PrimaryButton\"\nbase = \"B\"\nmodifier = \"primary\"\n",
    )
    .unwrap();
    let loaded = LoadedConfig { path: None, config };
    let unit = layout_expand("PrimaryButton\"Save\"", None, &loaded.customs().unwrap()).unwrap();
    assert!(unit.contains("Button::new"));
    assert!(unit.contains("ButtonVariant::Primary"));
}

#[test]
fn layout_commands_honor_explicit_config() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("toolkit.toml");
    std::fs::write(
        &config,
        "[[layout.components]]\nname = \"PrimaryButton\"\nbase = \"B\"\nmodifier = \"primary\"\n",
    )
    .unwrap();

    let cli = Cli::try_parse_from([
        "gpui-toolkit",
        "layout",
        "check",
        "PrimaryButton\"Save\"",
        "--config",
        config.to_str().unwrap(),
    ])
    .unwrap();
    let output = run_command(&cli).unwrap();
    let value: serde_json::Value = serde_json::from_str(&output.json).unwrap();
    assert_eq!(value["data"]["customComponents"][0], "PrimaryButton");

    let cli = Cli::try_parse_from([
        "gpui-toolkit",
        "layout",
        "expand",
        "PrimaryButton\"Save\"",
        "--config",
        config.to_str().unwrap(),
    ])
    .unwrap();
    let output = run_command(&cli).unwrap();
    assert!(output.text.contains("ButtonVariant::Primary"));

    let missing = dir.path().join("nope.toml");
    let cli = Cli::try_parse_from([
        "gpui-toolkit",
        "layout",
        "check",
        "B\"Hi\"",
        "--config",
        missing.to_str().unwrap(),
    ])
    .unwrap();
    let error = run_command(&cli).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidConfig);
}

#[test]
fn doctor_config_check_tracks_toolkit_toml() {
    use gpui_toolkit_cli::{DoctorStatus, run_doctor};

    let dir = tempfile::tempdir().unwrap();
    let report = run_doctor(dir.path());
    let check = report
        .checks
        .iter()
        .find(|check| check.id == "config")
        .unwrap();
    assert_eq!(check.status, DoctorStatus::Info);

    std::fs::write(
        dir.path().join("toolkit.toml"),
        "[project]\nname = \"demo\"\n",
    )
    .unwrap();
    let report = run_doctor(dir.path());
    let check = report
        .checks
        .iter()
        .find(|check| check.id == "config")
        .unwrap();
    assert_eq!(check.status, DoctorStatus::Ok);
    assert!(!report.failed());

    std::fs::write(dir.path().join("toolkit.toml"), "[bogus]\n").unwrap();
    let report = run_doctor(dir.path());
    let check = report
        .checks
        .iter()
        .find(|check| check.id == "config")
        .unwrap();
    assert_eq!(check.status, DoctorStatus::Fail);
    assert!(check.fix.is_some());
    assert!(report.failed());
}

#[test]
fn upgrade_detect_finds_calls_not_comments() {
    use gpui_toolkit_cli::{UPGRADE_DETECT_TYPE, render_upgrade_detect_text, upgrade_detect};

    assert_eq!(UPGRADE_DETECT_TYPE, "upgrade.detect");
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("lib.rs"),
        "fn f() {\n    thing.build_with_theme(cx);\n    // build_with_theme( is deprecated\n    let url = \"https://x.test\"; other.build_with_theme(cx);\n}\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("notes.txt"), "build_with_theme(\n").unwrap();
    let nested = dir.path().join("sub");
    std::fs::create_dir(&nested).unwrap();
    std::fs::write(nested.join("m.rs"), "fn g() {}\n").unwrap();
    let report = upgrade_detect(dir.path(), &[]).unwrap();
    assert_eq!(report.scanned_files, 2);
    assert_eq!(report.count, 2);
    assert_eq!(report.findings[0].line, 2);
    assert_eq!(report.findings[1].line, 4);
    assert_eq!(report.findings[0].rule, "no-build-with-theme");
    assert!(report.findings[0].fix.is_some());
    let text = render_upgrade_detect_text(&report, false);
    assert!(text.contains("2 findings"));
    let dense = render_upgrade_detect_text(&report, true);
    assert!(dense.contains("findings=2"));

    let error = upgrade_detect(&dir.path().join("nope"), &[]).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgument);
}

#[test]
fn upgrade_detect_honors_config_rules() {
    use gpui_toolkit_cli::{LoadedConfig, parse_config, upgrade_detect};

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("lib.rs"), "legacy_color();\n").unwrap();
    let config = parse_config(
        "[[upgrade.rules]]\nid = \"no-legacy\"\npattern = \"legacy_\"\nmessage = \"gone\"\n",
    )
    .unwrap();
    let loaded = LoadedConfig { path: None, config };
    let report = upgrade_detect(dir.path(), &loaded.upgrade_rules().unwrap()).unwrap();
    assert_eq!(report.count, 1);
    assert_eq!(report.findings[0].rule, "no-legacy");
    assert_eq!(report.rules.len(), 2);
}

#[test]
fn upgrade_command_detect_exits_1_on_findings() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("lib.rs"), "x.build_with_theme(cx);\n").unwrap();
    let cli = Cli::try_parse_from([
        "gpui-toolkit",
        "upgrade",
        "--detect",
        "--dir",
        dir.path().to_str().unwrap(),
    ])
    .unwrap();
    let output = run_command(&cli).unwrap();
    assert_eq!(output.exit_code, 1);
    assert!(output.json.contains("\"type\": \"upgrade.detect\""));

    let clean = tempfile::tempdir().unwrap();
    std::fs::write(clean.path().join("lib.rs"), "fn f() {}\n").unwrap();
    let cli = Cli::try_parse_from([
        "gpui-toolkit",
        "upgrade",
        "--detect",
        "--dir",
        clean.path().to_str().unwrap(),
    ])
    .unwrap();
    let output = run_command(&cli).unwrap();
    assert_eq!(output.exit_code, 0);

    let cli = Cli::try_parse_from(["gpui-toolkit", "upgrade"]).unwrap();
    let output = run_command(&cli).unwrap();
    assert!(output.json.contains("\"type\": \"upgrade.list\""));
}

#[test]
fn gap_report_honors_configured_issues_url() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("toolkit.toml");
    std::fs::write(
        &config,
        "[project]\nissues_url = \"https://x.test/tracker/issues\"\n",
    )
    .unwrap();
    let cli = Cli::try_parse_from([
        "gpui-toolkit",
        "gap-report",
        "--area",
        "cli",
        "Title",
        "--config",
        config.to_str().unwrap(),
    ])
    .unwrap();
    let output = run_command(&cli).unwrap();
    assert!(output.text.contains("https://x.test/tracker/issues"));
}
