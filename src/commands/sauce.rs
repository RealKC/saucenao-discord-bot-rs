//https://fsa.zobj.net/crop.php?r=TTaoo-Hk-NzrmVjElOUhUzi89I-XZojwpmk_E8w3SClP7apqNrE-YEKqCtf_WJ5CeIk5IRVf8q8jfCUSXeRixiP12a25ZWPzHzbxUBjpF9iNixLG2V0TZRKjp4I3JV73bfV5vLEwmBn1W5-F

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
    let urls = match get_urls(&ctx, &msg, args) {
        Some(urls) => { urls }
        None => {
            call_user_out(ctx, msg).await?;
            return Ok(());
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

    for url in &urls {
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

fn get_urls(ctx: &Context, msg: &Message, args: Args) -> Option<Vec<URL>> {
    let mut res = if let Some(urls) = get_urls_from_message(msg) {
        urls
    } else { 
        Vec::new() 
    };

    if let Some(urls) = msg.referenced_message
        .clone().and_then(|m| get_attachment_urls(&*m)) {
            res.append(&urls);
        }

    if res.is_empty() {
        let prev_msg = msg
            .channel_id
            .messages(ctx, |gm| gm.before(msg.id).limit(1))
            .await?;

        if let Some(urls) = get_attachment_urls(prev_msg) {
            Some(urls)
        }
        else {
            None
        }
    } else {
        Some(res)
    }
}

fn get_urls_from_message(msg: &Message) -> Option<Vec<Url>> {
    let mut res = Vec::new();

    if let Some(urls) = get_attachment_urls(&msg) {
        res.append(&urls);
    }

    let raw_arg = args.parse::<String>().unwrap_or_else(|_| "".into());
    if let Some(urls) = get_urls_from_string(raw_arg) {
        res.append(&urls);
    }

    if res.empty() {
        None
    } else {
        Some(res)
    }
}

fn get_attachment_urls(msg: &Message) -> Option<Vec<Url>> {
    if msg.attachments.empty() {
        None
    } else {
        let mut res = Vec::new();
        for attachment in &msg.attachments {
            if let Ok(url) = Url::parse(&attachment.url) {
                res.push(url);
            }
        }

        if res.empty() {
            None
        } else {
            Some(res)
        }
    }
}

fn get_urls_from_string(s: &String) -> Option<Vec<Url>> {
    const MIN_URL_LENGTH: usize = "<http://>".len();

    if s.len() < MIN_URL_LENGTH {
        None
    } else {
        //NOTE: We will assume a valid URL can not contain spaces.
        let mut res = Vec::new();
        for Some(mut section) in s.split_whitespace() {
            if section.len() < MIN_URL_LENGTH {
                continue;
            }

            if section.starts_with('<') {
                section = section[1..].to_string();
            }

            if section.ends_with('>') {
                section = section[..raw_arg.len()].to_string();
            }


            if let Ok(url) = Url::parse(&section) {
                res.push(url);
            }
        }

        if res.empty() {
            None
        } else {
            Some(res)
        }
    }
}
