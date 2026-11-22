use dioxus::prelude::*;
use crate::api::fetch_tasks;
use crate::models::task::{Task, TaskStatus};

#[component]
pub fn KanbanBoard() -> Element {
    let mut tasks_data = use_signal(|| Vec::new());
    let mut loading = use_signal(|| true);

    use_future(move || async move {
        if let Ok(response) = fetch_tasks().await {
            tasks_data.set(response.tasks);
        }
        loading.set(false);
    });

    // Collect all pending tasks with document/header context
    let pending_items: Vec<(String, String, Task)> = {
        let mut items = Vec::new();
        for doc in tasks_data.iter() {
            let doc_title = doc.document_title.clone();
            for group in &doc.groups {
                let header = group.header.clone();
                for task in &group.tasks {
                    if task.status == TaskStatus::Pending {
                        items.push((doc_title.clone(), header.clone(), task.clone()));
                    }
                }
            }
        }
        items
    };

    // Collect all completed tasks with document/header context
    let completed_items: Vec<(String, String, Task)> = {
        let mut items = Vec::new();
        for doc in tasks_data.iter() {
            let doc_title = doc.document_title.clone();
            for group in &doc.groups {
                let header = group.header.clone();
                for task in &group.tasks {
                    if task.status == TaskStatus::Completed {
                        items.push((doc_title.clone(), header.clone(), task.clone()));
                    }
                }
            }
        }
        items
    };

    rsx! {
        div {
            class: "kanban-board",
            style: "display: flex; gap: 20px; padding: 20px;",
            
            // Pending column
            div {
                class: "kanban-column",
                style: "flex: 1; background: #ffffff; padding: 15px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                h2 { 
                    style: "color: #1f2937; margin-bottom: 15px;",
                    "Pending ({pending_items.len()})" 
                }
                if loading() {
                    p { "Loading..." }
                } else {
                    for (doc_title, header, task) in pending_items.iter() {
                        div {
                            key: "{doc_title}-{header}-{task.text}",
                            style: "background: #f8f9fa; padding: 10px; margin-bottom: 8px; border-radius: 4px; border-left: 3px solid #f59e0b;",
                            div { 
                                style: "font-size: 11px; color: #6b7280; margin-bottom: 4px;",
                                "{doc_title} › {header}"
                            }
                            div {
                                style: "color: #1f2937;",
                                "{task.text}"
                            }
                        }
                    }
                }
            }
            
            // Completed column
            div {
                class: "kanban-column",
                style: "flex: 1; background: #ffffff; padding: 15px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                h2 { 
                    style: "color: #1f2937; margin-bottom: 15px;",
                    "Completed ({completed_items.len()})" 
                }
                if loading() {
                    p { "Loading..." }
                } else {
                    for (doc_title, header, task) in completed_items.iter() {
                        div {
                            key: "{doc_title}-{header}-{task.text}",
                            style: "background: #f8f9fa; padding: 10px; margin-bottom: 8px; border-radius: 4px; border-left: 3px solid #10b981;",
                            div { 
                                style: "font-size: 11px; color: #6b7280; margin-bottom: 4px;",
                                "{doc_title} › {header}"
                            }
                            div {
                                style: "color: #6b7280; text-decoration: line-through;",
                                "{task.text}"
                            }
                        }
                    }
                }
            }
        }
    }
}
