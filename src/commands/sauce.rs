use std::cmp::min;

use sauce_api::Sauce;
use serenity::framework::standard::CommandResult;
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args},
    model::channel::{Message, ReactionType},
};
use tracing::info;
use url::Url;

use crate::SauceContainer;

#[command]
#[description("This command will search saucenao for the image you provide it with. Not providing one will make me check the first attachment of the previous message")]
pub async fn sauce(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let url = {
        let mut raw_arg = args.parse::<String>().unwrap_or_else(|_| "".into());
        if raw_arg.is_empty() {
            let message = msg
                .channel_id
                .messages(ctx, |gm| gm.before(msg.id).limit(1))
                .await?;

            let attachment = &message[0].attachments.get(0);
            if let Some(attachment) = attachment {
                Url::parse(&attachment.url)
                    .expect("Either Discord or serenity gave us an invalid URL!")
            } else {
                call_user_out(ctx, msg).await?;
                return Ok(());
            }
        } else {
            const MIN_URL_LENGTH: usize = "<http://>".len();

            if raw_arg.len() < MIN_URL_LENGTH {
                call_user_out(ctx, msg).await?;
                return Ok(());
            }

            if raw_arg.starts_with('<') {
                raw_arg = raw_arg[1..].to_string();
            }

            if raw_arg.ends_with('>') {
                raw_arg = raw_arg[..raw_arg.len()].to_string();
            }

            if let Ok(url) = Url::parse(&raw_arg) {
                url
            } else {
                call_user_out(ctx, msg).await?;
                return Ok(());
            }
        }
    };

    let saucenao_guard = ctx
        .data
        .read()
        .await
        .get::<SauceContainer>()
        .cloned()
        .expect("Bruh. Where's the Sauce");
    let saucenao = saucenao_guard.read().await;

    let sauce = saucenao.check_sauce(url.as_str().to_string()).await;

    match sauce {
        Ok(sauce) => {
            msg.react(ctx, ReactionType::Unicode("✅".into())).await?;
            info!("URL?: {}", sauce.original_url);

            let mut contents = String::with_capacity(2000);
            contents.push_str("Possible sauces:\n");
            for i in 0..min(5, sauce.items.len()) {
                contents.push_str(&format!(
                    "* {similarity}% similar: <{url}>\n",
                    similarity = sauce.items[i].similarity,
                    url = sauce.items[i].link
                ));
            }

            msg.author.dm(ctx, |m| m.content(contents)).await?;
        }
        Err(why) => {
            msg.react(ctx, ReactionType::Unicode("❌".into())).await?;
            msg.author
                .dm(ctx, |m| {
                    m.content(format!("Couldn't get the sauce :c: \n ```{}```", why))
                })
                .await?;
        }
    }

    Ok(())
}

#[inline]
async fn call_user_out(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "The message previous to yours must either have an attachment, or you must provide an URL as argument to the command").await?;
    Ok(())
}
