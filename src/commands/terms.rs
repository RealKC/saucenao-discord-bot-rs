use serenity::prelude::*;
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
};

#[command]
#[description("Shows the terms you agree to by using this bot")]
pub async fn terms(ctx: &Context, msg: &Message) -> CommandResult {
    let name = {
        let me = ctx.http.get_current_user().await?;
        me.name
    };

    msg.reply(ctx, format!(r#"
{0} does not store any personally identifiable information.

By using {0} you agree to not send any illegal content through the network in which this bot is hosted.
    "#, name)).await?;
    Ok(())
}
