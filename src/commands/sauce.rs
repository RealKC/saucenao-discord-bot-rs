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
pub async fn sauce(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let url = {
        let url = Url::parse(args.rest()).ok();
        if let Some(url) = url {
            url
        } else {
            let message = msg
                .channel_id
                .messages(ctx, |gm| gm.before(msg.id).limit(1))
                .await?;

            let url = &message[0].attachments[0].url;
            Url::parse(url).expect("Either Discord or serenity gave us an invalid URL!")
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
