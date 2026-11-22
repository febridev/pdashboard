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
        else if trimmed.starts_with("- [ ]")
            || trimmed.starts_with("- [x]")
            || trimmed.starts_with("- [X]")
        {
            let status = if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
                TaskStatus::Completed
            } else {
                TaskStatus::Pending
            };

            let text = if let Some(rest) = trimmed.strip_prefix("- [ ]") {
                rest.trim().to_string()
            } else if let Some(rest) = trimmed.strip_prefix("- [x]") {
                rest.trim().to_string()
            } else if let Some(rest) = trimmed.strip_prefix("- [X]") {
                rest.trim().to_string()
            } else {
                continue; // shouldn't happen, but safe fallback
            };

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
