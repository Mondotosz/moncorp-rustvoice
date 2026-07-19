use crate::{Context, Error};

/// Extension methods for [`Context`] to reduce reply boilerplate.
pub(crate) trait ContextExt {
    async fn say_ephemeral(&self, content: impl Into<String> + Send) -> Result<(), Error>;
}

impl ContextExt for Context<'_> {
    async fn say_ephemeral(&self, content: impl Into<String> + Send) -> Result<(), Error> {
        self.send(
            poise::CreateReply::default()
                .content(content.into())
                .ephemeral(true),
        )
        .await?;
        Ok(())
    }
}
