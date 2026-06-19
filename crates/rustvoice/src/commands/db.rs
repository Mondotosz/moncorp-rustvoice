use crate::cli::DbAction;

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn run(action: DbAction) -> Result<(), Error> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./db.sqlite".into());

    let db = db::connection::connect_raw(&url).await?;

    match action {
        DbAction::Status => {
            db::management::status(&db).await?;
        }
        DbAction::Fresh => {
            db::management::fresh(&db).await?;
            println!("Database freshly migrated.");
        }
        DbAction::Refresh => {
            db::management::refresh(&db).await?;
            println!("Database refreshed.");
        }
        DbAction::Reset => {
            db::management::reset(&db).await?;
            println!("All migrations rolled back.");
        }
        DbAction::Up { num } => {
            db::management::up(&db, num).await?;
            println!("Migrations applied.");
        }
        DbAction::Down { num } => {
            db::management::down(&db, Some(num)).await?;
            println!("Rolled back {num} migration(s).");
        }
    }
    Ok(())
}
