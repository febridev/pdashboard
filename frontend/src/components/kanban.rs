use dioxus::prelude::*;
use crate::api::{fetch_tasks, ApiError};
use crate::models::task::{Task, TaskStatus};

#[component]
pub fn KanbanBoard() -> Element {
    let mut tasks_data = use_signal(|| Vec::new());
    let mut loading = use_signal(|| true);
    let mut authenticated = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);

    use_future(move || async move {
        match fetch_tasks().await {
            Ok(response) => {
                tasks_data.set(response.tasks);
                authenticated.set(true);
                loading.set(false);
            }
            Err(ApiError::Unauthorized) => {
                authenticated.set(false);
                loading.set(false);
            }
            Err(ApiError::Other(e)) => {
                error.set(Some(e));
                loading.set(false);
            }
        }
    });

    // Show real errors visibly: a failed request (network, proxy, parsing...)
    // must never masquerade as "please sign in".
    if let Some(err) = error() {
        return rsx! {
            div {
                style: "display: flex; justify-content: center; align-items: center; min-height: 400px;",
                div {
                    style: "background: #ffffff; padding: 40px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); text-align: center; max-width: 480px;",
                    h2 {
                        style: "color: #ef4444; margin-bottom: 20px;",
                        "Failed to load tasks"
                    }
                    p {
                        style: "color: #6b7280; margin-bottom: 30px; font-size: 14px; word-break: break-word;",
                        "{err}"
                    }
                }
            }
        };
    }

    // Show login button only on a genuine 401 from the API
    if !authenticated() && !loading() {
        return rsx! {
            div {
                style: "display: flex; justify-content: center; align-items: center; min-height: 400px;",
                div {
                    style: "background: #ffffff; padding: 40px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); text-align: center;",
                    h2 { 
                        style: "color: #1f2937; margin-bottom: 20px;",
                        "Personal Dashboard" 
                    }
                    p { 
                        style: "color: #6b7280; margin-bottom: 30px;",
                        "Please sign in to view your tasks" 
                    }
                    a {
                        href: "/auth/google",
                        style: "background: #3b82f6; color: white; padding: 12px 24px; border-radius: 6px; text-decoration: none; font-weight: 500;",
                        "Sign in with Google"
                    }
                }
            }
        };
    }

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
            // User status bar
            div {
                style: "display: flex; justify-content: space-between; align-items: center; padding: 10px 20px; background: #ffffff; border-radius: 8px; margin-bottom: 20px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                div {
                    style: "display: flex; align-items: center; gap: 8px;",
                    div {
                        style: "width: 8px; height: 8px; background: #10b981; border-radius: 50%;",
                    }
                    span {
                        style: "color: #6b7280; font-size: 14px;",
                        "Signed in"
                    }
                }
                a {
                    href: "/auth/logout",
                    style: "color: #6b7280; font-size: 14px; text-decoration: none;",
                    "Sign out"
                }
            }

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
}
