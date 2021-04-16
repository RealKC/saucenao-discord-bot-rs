use serenity::{framework::standard::Args, prelude::*};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
    model::prelude::*,
};

#[command]
#[owners_only]
#[only_in(dms)]
pub async fn speak(ctx: &Context, _: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;

    channel_id.say(ctx, args.rest()).await?;

    Ok(())
}
