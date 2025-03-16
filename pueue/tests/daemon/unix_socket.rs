#[cfg(not(target_os = "windows"))]
mod tests {
    use ciborium::{from_reader, into_writer};
    use color_eyre::Result;
    use pretty_assertions::assert_eq;
    use tokio::task;

    use crate::helper::daemon_base_setup;
    use pueue::daemon::network::socket::get_listener;
    use pueue_lib::{message::*, network::protocol::*};

    /// This tests whether we can create a listener and client, that communicate via unix sockets.
    #[tokio::test]
    async fn test_unix_socket() -> Result<()> {
        better_panic::install();
        let (settings, _tempdir) = daemon_base_setup()?;

        let listener = get_listener(&settings.shared).await?;
        let message = Request::Status;

        let mut original_bytes = Vec::new();
        into_writer(&message, &mut original_bytes).expect("Failed to serialize message.");

        // Spawn a sub thread that:
        // 1. Accepts a new connection
        // 2. Reads a message
        // 3. Sends the same message back
        task::spawn(async move {
            let mut stream = listener.accept().await.unwrap();
            let message_bytes = receive_bytes(&mut stream).await.unwrap();

            let message: Request = from_reader(message_bytes.as_slice()).unwrap();

            send_request(message, &mut stream).await.unwrap();
        });

        let mut client = get_client_stream(settings.shared.clone().try_into()?).await?;

        // Create a client that sends a message and instantly receives it
        send_request(message, &mut client).await?;
        let response_bytes = receive_bytes(&mut client).await?;
        let _message: Request = from_reader(response_bytes.as_slice())?;

        assert_eq!(response_bytes, original_bytes);

        Ok(())
    }
}
