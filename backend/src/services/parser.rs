use crate::models::task::{Task, TaskGroup, TaskStatus};

pub fn parse_tasks_from_markdown(markdown: &str) -> Vec<TaskGroup> {
    let mut groups: Vec<TaskGroup> = Vec::new();
    let mut heading_stack: Vec<(usize, String)> = Vec::new();
    let mut current_tasks: Vec<Task> = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim();

        // Check for header (H1-H6)
        if trimmed.starts_with('#') {
            // Save previous group if exists
            if !current_tasks.is_empty() && !heading_stack.is_empty() {
                let (header, path) = current_heading(&heading_stack);
                groups.push(TaskGroup {
                    header,
                    path,
                    tasks: current_tasks.clone(),
                });
                current_tasks.clear();
            }

            // count leading '#' as level, rest is the heading text
            let level = trimmed.chars().take_while(|c| *c == '#').count();
            let text = trimmed.trim_start_matches('#').trim().to_string();
            // pop same-or-shallower headings, then push
            heading_stack.retain(|(lvl, _)| *lvl < level);
            heading_stack.push((level, text));
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
    if !current_tasks.is_empty() && !heading_stack.is_empty() {
        let (header, path) = current_heading(&heading_stack);
        groups.push(TaskGroup {
            header,
            path,
            tasks: current_tasks,
        });
    }

    groups
}

// Header is the immediate heading; path is the full stack root→leaf.
fn current_heading(heading_stack: &[(usize, String)]) -> (String, Vec<String>) {
    (
        heading_stack.last().unwrap().1.clone(),
        heading_stack.iter().map(|(_, t)| t.clone()).collect(),
    )
}
