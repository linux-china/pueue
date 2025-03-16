use chrono::Local;
use pueue_lib::{GroupStatus, Settings, Task, TaskStatus, failure_msg, message::*};

use crate::{
    aliasing::insert_alias,
    daemon::{
        internal_state::SharedState,
        network::{message_handler::ok_or_failure_message, response_helper::ensure_group_exists},
        process_handler,
    },
    ok_or_save_state_failure,
};

#[cfg(doc)]
use pueue_lib::State;

/// Invoked when calling `pueue add`.
/// Queues a new [Task] to the [State].
/// If the start_immediately flag is set, send a StartMessage to the task handler.
pub fn add_task(settings: &Settings, state: &SharedState, message: AddRequest) -> Response {
    let mut state = state.lock().unwrap();
    if let Err(response) = ensure_group_exists(&mut state, &message.group) {
        return response;
    }

    // Ensure that specified dependencies actually exist.
    let not_found: Vec<_> = message
        .dependencies
        .iter()
        .filter(|id| !state.tasks().contains_key(id))
        .collect();
    if !not_found.is_empty() {
        return failure_msg!("Unable to setup dependencies : task(s) {not_found:?} not found",);
    }

    // Create a new task and add it to the state.
    let mut task = Task::new(
        message.command,
        message.path,
        message.envs,
        message.group,
        TaskStatus::Queued {
            enqueued_at: Local::now(),
        },
        message.dependencies,
        message.priority.unwrap_or(0),
        message.label,
    );

    // Handle if the command is to be stashed and/or automatically enqueued later.
    if message.stashed || message.enqueue_at.is_some() {
        task.status = TaskStatus::Stashed {
            enqueue_at: message.enqueue_at,
        };
    }

    // Check if there're any aliases that should be applied.
    // If one is found, we expand the command, otherwise we just take the original command.
    // We save the aliased command separately and keep the original command in its own field.
    //
    // This is especially important when editing commands as users should edit the original
    // command (for instance if they made a typo and used the wrong alias).
    task.command = insert_alias(settings, task.original_command.clone());

    // Sort and deduplicate dependency ids.
    task.dependencies.sort_unstable();
    task.dependencies.dedup();

    // Check if the task's group is paused before we pass it to the state
    let group_status = state
        .groups()
        .get(&task.group)
        .expect("We ensured that the group exists.")
        .status;
    let group_is_paused = matches!(group_status, GroupStatus::Paused);

    // Add the task and persist the state.
    let task_id = state.add_task(task);
    ok_or_save_state_failure!(state.save(settings));

    // Notify the task handler, in case the client wants to start the task immediately.
    if message.start_immediately {
        process_handler::start::start(settings, &mut state, TaskSelection::TaskIds(vec![task_id]));
    }

    AddedTaskResponse {
        task_id,
        enqueue_at: message.enqueue_at,
        group_is_paused,
    }
    .into()
}
