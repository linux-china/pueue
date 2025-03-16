use pueue_lib::{Client, Settings, message::*};

use super::{handle_response, handle_user_confirmation};
use crate::{client::style::OutputStyle, internal_prelude::*};

/// Tell the daemon to remove some tasks.
pub async fn remove(
    client: &mut Client,
    settings: Settings,
    style: &OutputStyle,
    task_ids: Vec<usize>,
) -> Result<()> {
    if settings.client.show_confirmation_questions {
        handle_user_confirmation("remove", &task_ids)?;
    }
    client.send_request(Request::Remove(task_ids)).await?;

    let response = client.receive_response().await?;

    handle_response(style, response)
}
