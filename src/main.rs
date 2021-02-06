mod commands;
mod hooks;

use commands::*;
use hooks::*;

use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::standard::{buckets::LimitedFor, StandardFramework},
    http::Http,
    model::gateway::Ready,
};
use std::{collections::HashSet, env, sync::Arc};

use serenity::prelude::*;

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new_with_token(&token);

    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            if let Some(team) = info.team {
                owners.insert(team.owner_user_id);
            } else {
                owners.insert(info.owner.id);
            }
            match http.get_current_user().await {
                Ok(bot_id) => (owners, bot_id.id),
                Err(why) => panic!("Could not access the bot id: {:?}", why),
            }
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let framework = StandardFramework::new()
        .configure(|c| {
            c.with_whitespace(true)
                .on_mention(Some(bot_id))
                .prefix("~")
                .delimiters(vec![", ", ","])
                .owners(owners)
        })
        .after(after)
        .unrecognised_command(unknown_command)
        .normal_message(normal_message)
        .bucket("emoji", |b| b.delay(5))
        .await
        .bucket("complicated", |b| {
            b.limit(2)
                .time_span(30)
                .delay(5)
                .limit_for(LimitedFor::Channel)
                .await_ratelimits(1)
                .delay_action(delay_action)
        })
        .await
        .help(&MY_HELP);
    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
    }

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
