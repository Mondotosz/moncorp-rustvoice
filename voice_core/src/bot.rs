mod commands;
use crate::db::DB;
use log::{debug, error, info};
use poise::serenity_prelude as serenity;

pub struct Data {
    pub db: DB,
} // User data, which is stored and accessible in all command invocations
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // There are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            error!("Error in command: `{}`: {:?}", ctx.command().name, error);
        }

        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling err: {}", e)
            }
        }
    }
}

pub struct Bot {
    client: serenity::Client,
}

impl Bot {
    pub async fn new(token: String, db: DB) -> Result<Bot, Error> {
        let intents = serenity::GatewayIntents::privileged()
            | serenity::GatewayIntents::GUILDS
            | serenity::GatewayIntents::GUILD_VOICE_STATES
            | serenity::GatewayIntents::GUILD_MESSAGES
            | serenity::GatewayIntents::GUILD_MESSAGE_REACTIONS;

        let commands = vec![
            commands::utils::ping(),
            commands::auto_voice::create(),
            commands::auto_voice::rename(),
            commands::auto_voice::private(),
            commands::auto_voice::public(),
            commands::auto_voice::limit(),
            commands::auto_voice::unlimit(),
        ];

        let framework: poise::Framework<Data, Error> = poise::Framework::builder()
            .options(poise::FrameworkOptions {
                commands,
                on_error: |e| Box::pin(on_error(e)),
                pre_command: |ctx| {
                    Box::pin(async move {
                        info!("Executing command {}...", ctx.command().qualified_name)
                    })
                },
                post_command: |ctx| {
                    Box::pin(async move {
                        info!("Executed command {}...", ctx.command().qualified_name)
                    })
                },
                skip_checks_for_owners: true,
                event_handler: |_ctx, event: &serenity::FullEvent, _framework, _data| {
                    Box::pin(async move {
                        match event {
                            serenity::FullEvent::VoiceStateUpdate { old, new } => {
                                commands::auto_voice::update_channels(_ctx, _data, old, new)
                                    .await?;
                                info!("Voice State changed");
                                Ok(())
                            }
                            _ => {
                                info!("Unused event triggered: {}", event.snake_case_name());
                                debug!("Unused event triggered: {:?}", event);
                                Ok(())
                            }
                        }
                    })
                },
                ..Default::default()
            })
            .setup(|ctx, _ready, framework| {
                Box::pin(async move {
                    info!("logged in as {}", _ready.user.name);
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    Ok(Data { db })
                })
            })
            .build();

        let client = serenity::ClientBuilder::new(token, intents)
            .framework(framework)
            .await?;

        Ok(Bot { client })
    }

    pub async fn start(&mut self) -> Result<(), serenity::Error> {
        self.client.start().await
    }
}
