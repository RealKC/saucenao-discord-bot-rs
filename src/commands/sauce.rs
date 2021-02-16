//https://fsa.zobj.net/crop.php?r=TTaoo-Hk-NzrmVjElOUhUzi89I-XZojwpmk_E8w3SClP7apqNrE-YEKqCtf_WJ5CeIk5IRVf8q8jfCUSXeRixiP12a25ZWPzHzbxUBjpF9iNixLG2V0TZRKjp4I3JV73bfV5vLEwmBn1W5-F

use std::cmp::min;

use sauce_api::Sauce;
use serenity::framework::standard::CommandResult;
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args},
    model::channel::{Message, ReactionType},
};
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
    let prev_msg = msg
        .channel_id
        .messages(ctx, |gm| gm.before(msg.id).limit(1))
        .await?;

    let urls = match get_urls(&msg, args, prev_msg.get(0)) {
        Some(urls) => urls,
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

    let mut react = React::Succeeded;

    for url in &urls {
        let sauce = saucenao.check_sauce(url.as_str()).await;

        match sauce {
            Ok(mut sauce) => {
                sauce.items.sort_by(|s1, s2| {
                    s1.partial_cmp(s2)
                        .expect("Saucenao gave us NaN similarities")
                });

                if sauce.items[0].similarity < 50.0 {
                    react = react.combine(React::HadIssues);
                }

                let mut contents = String::with_capacity(2000);
                contents.push_str(&format!("Possible sauces for <{url}>:\n", url = &url));
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
                react = react.combine(React::Failed);
                msg.author
                    .dm(ctx, |m| {
                        m.content(format!(
                            "Couldn't get the sauce for {url} :c: \n ```{why}```",
                            url = url,
                            why = why
                        ))
                    })
                    .await?;
            }
        }
    }

    msg.react(ctx, react.into_reaction_type()).await?;

    Ok(())
}

#[derive(PartialEq)]
enum React {
    Succeeded,
    HadIssues,
    Failed,
}

impl React {
    fn combine(self, other: React) -> Self {
        use React::*;

        match other {
            Succeeded => {
                if self == Succeeded {
                    Succeeded
                } else {
                    HadIssues
                }
            }
            HadIssues => HadIssues,
            Failed => {
                if self == Failed {
                    Failed
                } else {
                    HadIssues
                }
            }
        }
    }

    fn into_reaction_type(self) -> ReactionType {
        match self {
            React::Succeeded => ReactionType::Unicode("✅".into()),
            React::HadIssues => ReactionType::Unicode("😕".into()),
            React::Failed => ReactionType::Unicode("❌".into()),
        }
    }
}

async fn call_user_out(ctx: &Context, msg: &Message) -> CommandResult {
    msg.author.dm(ctx, |m| {
            m
            .content("One of your message or the previous message must have an attachment **or** you must provide a valid URL as argument to the command")
            }).await?;
    msg.react(ctx, ReactionType::Unicode("❌".into())).await?;
    Ok(())
}

fn get_urls(msg: &Message, args: Args, prev_msg: Option<&Message>) -> Option<Vec<Url>> {
    //Retrieve both URLs written down and the URLs from the attachments
    let mut res = if let Some(urls) = get_urls_from_message(msg, args) {
        urls
    } else {
        Vec::new()
    };

    //If the referenced message has attachments, add those URLs as well
    if let Some(mut urls) = msg
        .referenced_message
        .clone()
        .and_then(|m| get_attachment_urls(&*m))
    {
        res.append(&mut urls);
    }

    //If up to this point there are no URLs, try adding the attachments from the previous message
    if res.is_empty() {
        match prev_msg {
            Some(msg) => get_attachment_urls(msg),
            None => None,
        }
    } else {
        Some(res)
    }
}

fn get_urls_from_message(msg: &Message, args: Args) -> Option<Vec<Url>> {
    let mut res = Vec::new();

    if let Some(mut urls) = get_attachment_urls(&msg) {
        res.append(&mut urls);
    }

    let raw_arg = args.parse::<String>().unwrap_or_else(|_| "".into());
    if let Some(mut urls) = get_urls_from_string(&raw_arg) {
        res.append(&mut urls);
    }

    if res.is_empty() {
        None
    } else {
        Some(res)
    }
}

fn get_attachment_urls(msg: &Message) -> Option<Vec<Url>> {
    if msg.attachments.is_empty() {
        None
    } else {
        let mut res = Vec::new();
        for attachment in &msg.attachments {
            if let Ok(url) = Url::parse(&attachment.url) {
                res.push(url);
            }
        }

        if res.is_empty() {
            None
        } else {
            Some(res)
        }
    }
}

fn get_urls_from_string(s: &str) -> Option<Vec<Url>> {
    const MIN_URL_LENGTH: usize = "<http://>".len();

    if s.len() < MIN_URL_LENGTH {
        None
    } else {
        //NOTE: We will assume a valid URL can not contain spaces.
        let mut res = Vec::new();
        for mut section in s.split_whitespace() {
            if section.len() < MIN_URL_LENGTH {
                continue;
            }

            if section.starts_with('<') {
                section = &section[1..];
            }

            if section.ends_with('>') {
                section = &section[..section.len()];
            }

            if let Ok(url) = Url::parse(&section) {
                res.push(url);
            }
        }

        if res.is_empty() {
            None
        } else {
            Some(res)
        }
    }
}
