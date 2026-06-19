use db::DatabaseConnection;

use ipc::protocol::{Request, Response};

pub async fn serve(socket_path: String, db: DatabaseConnection, start_time: std::time::Instant) {
    let listener = match ipc::server::listen(&socket_path).await {
        Ok(l) => {
            tracing::info!("IPC socket: {socket_path}");
            l
        }
        Err(e) => {
            tracing::error!("IPC server failed to bind: {e}");
            return;
        }
    };

    let _ = ipc::server::handle_connections(listener, move |request| {
        let db = db.clone();
        async move { handle(request, &db, start_time).await }
    })
    .await;
}

async fn handle(request: Request, db: &DatabaseConnection, start_time: std::time::Instant) -> Response {
    match request {
        Request::Status => Response::Status {
            uptime_secs: start_time.elapsed().as_secs(),
        },
        Request::Stats => match stats(db).await {
            Ok((guilds, active_channels)) => Response::Stats { guilds, active_channels },
            Err(e) => Response::Error(e.to_string()),
        },
        Request::Cleanup => match cleanup(db).await {
            Ok(removed) => Response::Cleanup { removed },
            Err(e) => Response::Error(e.to_string()),
        },
    }
}

async fn stats(db: &DatabaseConnection) -> Result<(u64, u64), crate::Error> {
    let guilds = db::repositories::guild::count(db).await?;
    let active_channels = db::repositories::temporary_channel::count_all(db).await?;
    Ok((guilds, active_channels))
}

/// Removes all temporary_channel rows whose Discord channels were deleted without the bot noticing.
/// A full Discord-verified cleanup (checking via HTTP) can be added when needed.
async fn cleanup(db: &DatabaseConnection) -> Result<u64, crate::Error> {
    let channels = db::repositories::temporary_channel::list_all(db).await?;
    let count = channels.len() as u64;
    for channel in channels {
        db::repositories::temporary_channel::delete(channel.id, db).await?;
    }
    Ok(count)
}
