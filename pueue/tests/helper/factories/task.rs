use pueue_lib::{message::*, settings::*};

use crate::helper::*;

/// Adds a task to the test daemon.
pub async fn add_task(shared: &Shared, command: &str) -> Result<Response> {
    send_request(shared, create_add_message(shared, command))
        .await
        .context("Failed to to add task.")
}

/// Adds a task to the test daemon and starts it immediately.
pub async fn add_and_start_task(shared: &Shared, command: &str) -> Result<Response> {
    let mut message = create_add_message(shared, command);
    message.start_immediately = true;

    send_request(shared, message)
        .await
        .context("Failed to to add task.")
}

/// Adds a task to the test daemon.
pub async fn add_task_with_priority(
    shared: &Shared,
    command: &str,
    priority: i32,
) -> Result<Response> {
    let mut message = create_add_message(shared, command);
    message.priority = Some(priority);

    send_request(shared, message)
        .await
        .context("Failed to to add task.")
}

/// Adds a task with a dependency to the test daemon.
pub async fn add_task_with_dependencies(
    shared: &Shared,
    command: &str,
    dependencies: Vec<usize>,
) -> Result<Response> {
    let mut message = create_add_message(shared, command);
    message.dependencies = dependencies;

    send_request(shared, message)
        .await
        .context("Failed to to add task.")
}

/// Adds a task to a specific group of the test daemon.
pub async fn add_task_to_group<C: ToString, G: ToString>(
    shared: &Shared,
    command: C,
    group: G,
) -> Result<Response> {
    let mut message = create_add_message(shared, command);
    message.group = group.to_string();

    send_request(shared, message)
        .await
        .context("Failed to to add task to group.")
}

/// Mini wrapper around add_task, which creates a task that echos PUEUE's worker environment
/// variables to `stdout`.
pub async fn add_env_task(shared: &Shared, command: &str) -> Result<Response> {
    let command = format!("echo WORKER_ID: $PUEUE_WORKER_ID; echo GROUP: $PUEUE_GROUP; {command}");
    add_task(shared, &command).await
}

/// Just like [add_env_task], but the task get's added to specific group.
pub async fn add_env_task_to_group(
    shared: &Shared,
    command: &str,
    group: &str,
) -> Result<Response> {
    let command = format!("echo WORKER_ID: $PUEUE_WORKER_ID; echo GROUP: $PUEUE_GROUP; {command}");
    add_task_to_group(shared, &command, group).await
}
