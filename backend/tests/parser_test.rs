use pdashboard_backend::models::task::TaskStatus;
use pdashboard_backend::services::parser::parse_tasks_from_markdown;

#[test]
fn test_parse_tasks_with_headers() {
    let markdown = r#"# Project Alpha

## TODO
- [ ] Setup database
- [ ] Design API

## Completed
- [x] Initial planning
- [x] Send emails
"#;

    let groups = parse_tasks_from_markdown(markdown);

    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].header, "TODO");
    assert_eq!(groups[0].path, vec!["Project Alpha".to_string(), "TODO".to_string()]);
    assert_eq!(groups[0].tasks.len(), 2);
    assert_eq!(groups[0].tasks[0].text, "Setup database");
    assert_eq!(groups[0].tasks[0].status, TaskStatus::Pending);

    assert_eq!(groups[1].header, "Completed");
    assert_eq!(groups[1].tasks.len(), 2);
    assert_eq!(groups[1].tasks[0].status, TaskStatus::Completed);
}

#[test]
fn test_nested_headings_build_path() {
    let markdown = "## smthk-amdb\n\n### 2026-11-23\n- [ ] do X\n\n### 2026-11-25\n- [ ] do Y\n\n## other\n- [ ] do Z\n";
    let groups = parse_tasks_from_markdown(markdown);
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].header, "2026-11-23");
    assert_eq!(groups[0].path, vec!["smthk-amdb".to_string(), "2026-11-23".to_string()]);
    assert_eq!(groups[1].header, "2026-11-25");
    assert_eq!(groups[1].path, vec!["smthk-amdb".to_string(), "2026-11-25".to_string()]);
    assert_eq!(groups[2].header, "other");
    assert_eq!(groups[2].path, vec!["other".to_string()]); // same level popped smthk-amdb — no nesting
}

#[test]
fn test_tasks_directly_under_name_heading_keep_short_path() {
    let markdown = "## smthk-amdb\n- [ ] do W\n\n### 2026-11-23\n- [ ] do X\n";
    let groups = parse_tasks_from_markdown(markdown);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].header, "smthk-amdb");
    assert_eq!(groups[0].path, vec!["smthk-amdb".to_string()]);
    assert_eq!(groups[1].path, vec!["smthk-amdb".to_string(), "2026-11-23".to_string()]);
}
