use serenity::prelude::*;
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
};

#[command]
pub async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    let info = ctx.http.get_current_application_info().await?;
    let owner = if let Some(team) = info.team {
        team.owner_user_id
    } else {
        info.owner.id
    };
    let owner = owner.to_user(ctx).await?;

    let bot_name = {
        let me = ctx.http.get_current_user().await?;
        me.name
    };

    msg.reply(ctx, format!(r#"
{bot_name} was written by {owner_name}#{owner_discrim} using the Rust language.

{bot_name} is licensed under the AGPLv3 license, and you may view its source code at <https://github.com/RealKC/saucenao-discord-bot-rs>.
    "#,
    bot_name=bot_name,
    owner_name=owner.name,
    owner_discrim=owner.discriminator
    )).await?;

    Ok(())
}
