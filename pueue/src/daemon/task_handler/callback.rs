use handlebars::RenderError;

use super::*;

impl TaskHandler {
    /// Users can specify a callback that's fired whenever a task finishes.
    /// Execute the callback by spawning a new subprocess.
    pub fn spawn_callback(&mut self, task: &Task) {
        // Return early, if there's no callback specified
        let Some(template_string) = &self.settings.daemon.callback else {
            return;
        };

        // Build the command to be called from the template string in the configuration file.
        let callback_command = match self.build_callback_command(task, template_string) {
            Ok(callback_command) => callback_command,
            Err(err) => {
                error!("Failed to create callback command from template with error: {err}");
                return;
            }
        };

        let mut command = compile_shell_command(&self.settings, &callback_command);

        // Spawn the callback subprocess and log if it fails.
        let spawn_result = command.spawn();
        let child = match spawn_result {
            Err(error) => {
                error!("Failed to spawn callback with error: {error}");
                return;
            }
            Ok(child) => child,
        };

        debug!("Spawned callback for task {}", task.id);
        self.callbacks.push(child);
    }

    /// Take the callback template string from the configuration and insert all parameters from the
    /// finished task.
    pub fn build_callback_command(
        &self,
        task: &Task,
        template_string: &str,
    ) -> Result<String, RenderError> {
        // Init Handlebars. We set to strict, as we want to show an error on missing variables.
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);

        // Add templating variables.
        let mut parameters = HashMap::new();
        parameters.insert("id", task.id.to_string());
        parameters.insert("command", task.command.clone());
        parameters.insert("path", (*task.path.to_string_lossy()).to_owned());
        parameters.insert("group", task.group.clone());
        parameters.insert("label", task.label.clone().unwrap_or("".to_owned()));

        // Result takes the TaskResult Enum strings, unless it didn't finish yet.
        if let TaskStatus::Done(result) = &task.status {
            parameters.insert("result", result.to_string());
        } else {
            parameters.insert("result", "None".into());
        }

        // Format and insert start and end times.
        let print_time = |time: Option<DateTime<Local>>| {
            time.map(|time| time.timestamp().to_string())
                .unwrap_or_default()
        };
        parameters.insert("start", print_time(task.start));
        parameters.insert("end", print_time(task.end));

        // Read the last lines of the process' output and make it available.
        if let Ok(output) = read_last_log_file_lines(
            task.id,
            &self.pueue_directory,
            self.settings.daemon.callback_log_lines,
        ) {
            parameters.insert("output", output);
        } else {
            parameters.insert("output", "".to_string());
        }

        let out_path = get_log_path(task.id, &self.pueue_directory);
        // Using Display impl of PathBuf which isn't necessarily a perfect
        // representation of the path but should work for most cases here
        parameters.insert("output_path", out_path.display().to_string());

        // Get the exit code
        if let TaskStatus::Done(result) = &task.status {
            match result {
                TaskResult::Success => parameters.insert("exit_code", "0".into()),
                TaskResult::Failed(code) => parameters.insert("exit_code", code.to_string()),
                _ => parameters.insert("exit_code", "None".into()),
            };
        } else {
            parameters.insert("exit_code", "None".into());
        }

        handlebars.render_template(template_string, &parameters)
    }

    /// Look at all running callbacks and log any errors.
    /// If everything went smoothly, simply remove them from the list.
    pub fn check_callbacks(&mut self) {
        let mut finished = Vec::new();
        for (id, child) in self.callbacks.iter_mut().enumerate() {
            match child.try_wait() {
                // Handle a child error.
                Err(error) => {
                    error!("Callback failed with error {error:?}");
                    finished.push(id);
                }
                // Child process did not exit yet.
                Ok(None) => continue,
                Ok(exit_status) => {
                    info!("Callback finished with exit code {exit_status:?}");
                    finished.push(id);
                }
            }
        }

        finished.reverse();
        for id in finished.iter() {
            self.callbacks.remove(*id);
        }
    }
}
