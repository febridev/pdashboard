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
    assert_eq!(groups[0].tasks.len(), 2);
    assert_eq!(groups[0].tasks[0].text, "Setup database");
    assert_eq!(groups[0].tasks[0].status, TaskStatus::Pending);

    assert_eq!(groups[1].header, "Completed");
    assert_eq!(groups[1].tasks.len(), 2);
    assert_eq!(groups[1].tasks[0].status, TaskStatus::Completed);
}
