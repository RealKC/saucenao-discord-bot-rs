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
#[description(r#"
This command searches Saucenao for the source of a provided image.

You may provide it in the following ways:
 \* by calling this command right after someone posted an image
 \* by calling this command while replying to a message with a image attachment
 \* by providing the URL of the image as a command parameter, for example `~sauce <url>`
 \* by calling this image in a message with an image attachment

It is important to note that this will NOT try to find URLs in the _contents_ of a previous message, it must be an attachment.
"#)]
pub async fn sauce(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let url = {
        let mut raw_arg = args.parse::<String>().unwrap_or_else(|_| "".into());
        if raw_arg.is_empty() {
            if let Some(url) = get_first_attachment_url_from(msg) {
                url
            } else if let Some(url) = msg
                .referenced_message
                .clone()
                .and_then(|m| get_first_attachment_url_from(&*m))
            {
                url
            } else {
                let message = msg
                    .channel_id
                    .messages(ctx, |gm| gm.before(msg.id).limit(1))
                    .await?;

                let url = message.get(0).and_then(get_first_attachment_url_from);

                if let Some(url) = url {
                    url
                } else {
                    call_user_out(ctx, msg).await?;
                    return Ok(());
                }
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

    let sauce = saucenao.check_sauce(url.as_str()).await;

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

async fn call_user_out(ctx: &Context, msg: &Message) -> CommandResult {
    msg.author.dm(ctx, |m| {
        m
        .content("One of your message or the previous message must have an attachment **or** you must provide a valid URL as argument to the command")
    }).await?;
    msg.react(ctx, ReactionType::Unicode("❌".into())).await?;
    Ok(())
}

fn get_first_attachment_url_from(msg: &Message) -> Option<Url> {
    let attachment = msg.attachments.get(0)?;

    Some(Url::parse(&attachment.url).expect("Either Discord or serenity gave us an invalid URL!"))
}
