use dioxus::prelude::*;

#[component]
pub fn KanbanBoard() -> Element {
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
                    "Pending (2)"
                }
                div {
                    style: "background: #f8f9fa; padding: 10px; margin-bottom: 8px; border-radius: 4px; border-left: 3px solid #f59e0b;",
                    "Sample pending task 1"
                }
                div {
                    style: "background: #f8f9fa; padding: 10px; margin-bottom: 8px; border-radius: 4px; border-left: 3px solid #f59e0b;",
                    "Sample pending task 2"
                }
            }

            // Completed column
            div {
                class: "kanban-column",
                style: "flex: 1; background: #ffffff; padding: 15px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                h2 {
                    style: "color: #1f2937; margin-bottom: 15px;",
                    "Completed (1)"
                }
                div {
                    style: "background: #f8f9fa; padding: 10px; margin-bottom: 8px; border-radius: 4px; border-left: 3px solid #10b981; color: #6b7280; text-decoration: line-through;",
                    "Sample completed task"
                }
            }
        }
    }
}
