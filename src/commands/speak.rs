// https://img.stickers.cloud/packs/9c8c35ba-9159-4969-8354-f30ad1793cad/webp/088a4c0b-32fc-423d-8744-688b5b8f2a42.webp

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
    let channel_id = args.single::<u64>()?;

    ChannelId(channel_id).say(ctx, args.rest()).await?;

    Ok(())
}
