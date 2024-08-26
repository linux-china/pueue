use pueue_lib::failure_msg;
use pueue_lib::network::message::*;
use pueue_lib::settings::Settings;
use pueue_lib::state::SharedState;
use pueue_lib::task::TaskStatus;

use super::ok_or_failure_message;
use crate::daemon::state_helper::save_state;
use crate::ok_or_save_state_failure;

/// Invoked when calling `pueue switch`.
/// Switch the position of two tasks in the upcoming queue.
/// We have to ensure that those tasks are either `Queued` or `Stashed`
pub fn switch(settings: &Settings, state: &SharedState, message: SwitchMessage) -> Message {
    let mut state = state.lock().unwrap();

    let task_ids = [message.task_id_1, message.task_id_2];
    let filtered_tasks = state.filter_tasks(
        |task| {
            matches!(
                task.status,
                TaskStatus::Queued { .. } | TaskStatus::Stashed { .. }
            )
        },
        Some(task_ids.to_vec()),
    );
    if !filtered_tasks.non_matching_ids.is_empty() {
        return failure_msg!("Tasks have to be either queued or stashed.");
    }
    if task_ids[0] == task_ids[1] {
        return failure_msg!("You cannot switch a task with itself.");
    }

    // Get the tasks. Expect them to be there, since we found no mismatch
    let mut first_task = state.tasks.remove(&task_ids[0]).unwrap();
    let mut second_task = state.tasks.remove(&task_ids[1]).unwrap();

    // Switch task ids
    let first_id = first_task.id;
    let second_id = second_task.id;
    first_task.id = second_id;
    second_task.id = first_id;

    // Put tasks back in again
    state.tasks.insert(first_task.id, first_task);
    state.tasks.insert(second_task.id, second_task);

    for (_, task) in state.tasks.iter_mut() {
        // If the task depends on both, we can just keep it as it is.
        if task.dependencies.contains(&first_id) && task.dependencies.contains(&second_id) {
            continue;
        }

        // If one of the ids is in the task's dependency list, replace it with the other one.
        if let Some(old_id) = task.dependencies.iter_mut().find(|id| *id == &first_id) {
            *old_id = second_id;
            task.dependencies.sort_unstable();
        } else if let Some(old_id) = task.dependencies.iter_mut().find(|id| *id == &second_id) {
            *old_id = first_id;
            task.dependencies.sort_unstable();
        }
    }

    ok_or_save_state_failure!(save_state(&state, settings));
    create_success_message("Tasks have been switched")
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    use super::super::fixtures::*;
    use super::*;

    fn get_message(task_id_1: usize, task_id_2: usize) -> SwitchMessage {
        SwitchMessage {
            task_id_1,
            task_id_2,
        }
    }

    fn get_test_state() -> (SharedState, Settings, TempDir) {
        let (state, settings, tempdir) = get_state();

        {
            let mut state = state.lock().unwrap();
            let task = get_stub_task("0", StubStatus::Queued);
            state.add_task(task);

            let task = get_stub_task("1", StubStatus::Stashed { enqueue_at: None });
            state.add_task(task);

            let task = get_stub_task("2", StubStatus::Queued);
            state.add_task(task);

            let task = get_stub_task("3", StubStatus::Stashed { enqueue_at: None });
            state.add_task(task);

            let mut task = get_stub_task("4", StubStatus::Queued);
            task.dependencies = vec![0, 3];
            state.add_task(task);

            let mut task = get_stub_task("5", StubStatus::Stashed { enqueue_at: None });
            task.dependencies = vec![1];
            state.add_task(task);

            let mut task = get_stub_task("6", StubStatus::Queued);
            task.dependencies = vec![2, 3];
            state.add_task(task);
        }

        (state, settings, tempdir)
    }

    #[test]
    /// A normal switch between two id's works perfectly fine.
    fn switch_normal() {
        let (state, settings, _tempdir) = get_test_state();

        let message = switch(&settings, &state, get_message(1, 2));

        // Return message is correct
        assert!(matches!(message, Message::Success(_)));
        if let Message::Success(text) = message {
            assert_eq!(text, "Tasks have been switched");
        };

        let state = state.lock().unwrap();
        assert_eq!(state.tasks.get(&1).unwrap().command, "2");
        assert_eq!(state.tasks.get(&2).unwrap().command, "1");
    }

    #[test]
    /// Tasks cannot be switched with themselves.
    fn switch_task_with_itself() {
        let (state, settings, _tempdir) = get_test_state();

        let message = switch(&settings, &state, get_message(1, 1));

        // Return message is correct
        assert!(matches!(message, Message::Failure(_)));
        if let Message::Failure(text) = message {
            assert_eq!(text, "You cannot switch a task with itself.");
        };
    }

    #[test]
    /// If any task that is specified as dependency get's switched,
    /// all dependants need to be updated.
    fn switch_task_with_dependant() {
        let (state, settings, _tempdir) = get_test_state();

        switch(&settings, &state, get_message(0, 3));

        let state = state.lock().unwrap();
        assert_eq!(state.tasks.get(&4).unwrap().dependencies, vec![0, 3]);
    }

    #[test]
    /// A task with two dependencies shouldn't experience any change, if those two dependencies
    /// switched places.
    fn switch_double_dependency() {
        let (state, settings, _tempdir) = get_test_state();

        switch(&settings, &state, get_message(1, 2));

        let state = state.lock().unwrap();
        assert_eq!(state.tasks.get(&5).unwrap().dependencies, vec![2]);
        assert_eq!(state.tasks.get(&6).unwrap().dependencies, vec![1, 3]);
    }

    #[test]
    /// You can only switch tasks that are either stashed or queued.
    /// Everything else should result in an error message.
    fn switch_invalid() {
        let (state, settings, _tempdir) = get_state();

        let combinations: Vec<(usize, usize)> = vec![
            (0, 1), // Queued + Done
            (0, 3), // Queued + Stashed
            (0, 4), // Queued + Running
            (0, 5), // Queued + Paused
            (2, 1), // Stashed + Done
            (2, 3), // Stashed + Stashed
            (2, 4), // Stashed + Running
            (2, 5), // Stashed + Paused
        ];

        for ids in combinations {
            let message = switch(&settings, &state, get_message(ids.0, ids.1));

            // Assert, that we get a Failure message with the correct text.
            assert!(matches!(message, Message::Failure(_)));
            if let Message::Failure(text) = message {
                assert_eq!(text, "Tasks have to be either queued or stashed.");
            };
        }
    }
}
