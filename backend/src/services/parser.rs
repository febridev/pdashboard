use crate::models::task::{Task, TaskGroup, TaskStatus};

pub fn parse_tasks_from_markdown(markdown: &str) -> Vec<TaskGroup> {
    let mut groups: Vec<TaskGroup> = Vec::new();
    let mut current_header = String::new();
    let mut current_tasks: Vec<Task> = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim();

        // Check for header (H1-H6)
        if trimmed.starts_with('#') {
            // Save previous group if exists
            if !current_tasks.is_empty() && !current_header.is_empty() {
                groups.push(TaskGroup {
                    header: current_header.clone(),
                    tasks: current_tasks.clone(),
                });
                current_tasks.clear();
            }

            // Extract header text (remove # symbols)
            current_header = trimmed.trim_start_matches('#').trim().to_string();
        }
        // Check for task items
        else if trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]") {
            let status = if trimmed.starts_with("- [x]") {
                TaskStatus::Completed
            } else {
                TaskStatus::Pending
            };

            let text = trimmed
                .trim_start_matches("- [ ]")
                .trim_start_matches("- [x]")
                .trim()
                .to_string();

            current_tasks.push(Task { text, status });
        }
    }

    // Add final group
    if !current_tasks.is_empty() && !current_header.is_empty() {
        groups.push(TaskGroup {
            header: current_header,
            tasks: current_tasks,
        });
    }

    groups
}
