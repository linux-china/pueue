use anyhow::{bail, Result};
use assert_matches::assert_matches;

use pueue_lib::network::message::*;
use pueue_lib::settings::Shared;
use pueue_lib::state::State;

use super::send_message;

/// Assert that a message is a successful message.
pub fn assert_success(message: Message) {
    assert_matches!(
        message,
        Message::Success(_),
        "Expected to get SuccessMessage, got {message:?}",
    );
}

/// Assert that a message is a failure message.
pub fn assert_failure(message: Message) {
    assert_matches!(
        message,
        Message::Failure(_),
        "Expected to get FailureMessage, got {message:?}",
    );
}

/// Make sure the expected environment variables are set.
/// This also makes sure, the variables have properly been injected into the processes'
/// environment.
pub async fn assert_worker_envs(
    shared: &Shared,
    state: &State,
    task_id: usize,
    worker: usize,
    group: &str,
) -> Result<()> {
    let task = state.tasks.get(&task_id).unwrap();
    // Make sure the environment variables have been properly set.
    assert_eq!(
        task.envs.get("PUEUE_GROUP"),
        Some(&group.to_string()),
        "Worker group didn't match for task {task_id}",
    );
    assert_eq!(
        task.envs.get("PUEUE_WORKER_ID"),
        Some(&worker.to_string()),
        "Worker id hasn't been correctly set for task {task_id}",
    );

    // Get the log output for the task.
    let response = send_message(
        shared,
        LogRequestMessage {
            tasks: TaskSelection::TaskIds(vec![task_id]),
            send_logs: true,
            lines: None,
        },
    )
    .await?;

    let Message::LogResponse(message) = response else {
        bail!("Expected LogResponse got {response:?}")
    };

    // Make sure the PUEUE_WORKER_ID and PUEUE_GROUP variables are present in the output.
    // They're always printed as to the [add_env_task] function.
    let log = message
        .get(&task_id)
        .expect("Log should contain requested task.");

    let stdout = log.output.clone().unwrap();
    let output = String::from_utf8_lossy(&stdout);
    assert!(
        output.contains(&format!("WORKER_ID: {worker}")),
        "Output should contain worker id {worker} for task {task_id}. Got: {output}",
    );
    assert!(
        output.contains(&format!("GROUP: {group}")),
        "Output should contain worker group {group} for task {task_id}. Got: {output}",
    );

    Ok(())
}
