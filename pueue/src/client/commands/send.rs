use pueue_lib::{Client, message::*};

use super::handle_response;
use crate::{client::style::OutputStyle, internal_prelude::*};

/// Send some input to a running task.
pub async fn send(
    client: &mut Client,
    style: &OutputStyle,
    task_id: usize,
    input: String,
) -> Result<()> {
    client.send_request(SendRequest { task_id, input }).await?;

    let response = client.receive_response().await?;

    handle_response(style, response)
}
