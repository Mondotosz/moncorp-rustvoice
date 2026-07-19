use ipc::client::IpcClient;
use ipc::protocol::{Request, Response};

fn temp_socket_path(label: &str) -> String {
    std::env::temp_dir()
        .join(format!(
            "rustvoice_test_{label}_{}.sock",
            std::process::id()
        ))
        .to_string_lossy()
        .into_owned()
}

async fn handler(request: Request) -> Response {
    match request {
        Request::Status => Response::Status {
            uptime_secs: 42,
            discord_ok: true,
        },
        Request::Stats => Response::Stats {
            guilds: 1,
            active_channels: 2,
        },
        Request::Cleanup => Response::Cleanup { removed: 3 },
    }
}

#[tokio::test]
async fn client_and_server_round_trip_every_request_variant() {
    let socket_path = temp_socket_path("roundtrip");
    let listener = ipc::server::listen(&socket_path)
        .await
        .expect("bind unix socket");
    tokio::spawn(ipc::server::handle_connections(listener, handler));

    let mut client = IpcClient::connect(&socket_path)
        .await
        .expect("connect to socket");
    let response = client.send(Request::Status).await.unwrap();
    assert_eq!(
        response,
        Response::Status {
            uptime_secs: 42,
            discord_ok: true
        }
    );

    let mut client = IpcClient::connect(&socket_path).await.unwrap();
    let response = client.send(Request::Stats).await.unwrap();
    assert_eq!(
        response,
        Response::Stats {
            guilds: 1,
            active_channels: 2
        }
    );

    let mut client = IpcClient::connect(&socket_path).await.unwrap();
    let response = client.send(Request::Cleanup).await.unwrap();
    assert_eq!(response, Response::Cleanup { removed: 3 });

    let _ = std::fs::remove_file(&socket_path);
}

#[tokio::test]
async fn connecting_to_a_missing_socket_fails_with_a_clear_message() {
    let socket_path = temp_socket_path("missing");
    let _ = std::fs::remove_file(&socket_path);

    let result = IpcClient::connect(&socket_path).await;
    let Err(err) = result else {
        panic!("expected connecting to a missing socket to fail");
    };
    assert!(err.to_string().contains("not running"));
}
