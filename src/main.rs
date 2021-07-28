//https://upload.wikimedia.org/wikipedia/commons/1/1b/Neko_Wikipe-tan.svg

mod commands;
mod hooks;

use commands::*;
use hooks::*;

use dotenv::dotenv;
use sauce_api::sources::SauceNao;
use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::standard::{buckets::LimitedFor, macros::group, StandardFramework},
    http::Http,
    model::{gateway::Ready, id::GuildId, prelude::Activity},
    prelude::*,
};
use std::{collections::HashSet, env, sync::Arc};
use tracing::info;

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct SauceContainer;

impl TypeMapKey for SauceContainer {
    type Value = Arc<RwLock<SauceNao>>;
}

#[group("Commands")]
#[commands(sauce, about, terms, speak)]
struct Commands;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        ctx.set_activity(Activity::playing("~help")).await;
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

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
                .delimiters(vec![", ", ",", " "])
                .owners(owners)
        })
        .after(after)
        .unrecognised_command(unknown_command)
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
        .help(&MY_HELP)
        .group(&COMMANDS_GROUP);
    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));

        let mut saucenao = SauceNao::new();
        saucenao.set_api_key(env::var("SAUCENAO_API_KEY").expect("The purpose of this bot is to search SauceNao, I need a SauceNao API Key in an env var called 'SAUCENAO_API_KEY' to work."));
        data.insert::<SauceContainer>(Arc::new(RwLock::new(saucenao)));
    }

    #[cfg(unix)]
    {
        use tokio::{signal::unix::signal, signal::unix::SignalKind};

        let shard_manager = client.shard_manager.clone();

        let signals_to_handle = vec![
            SignalKind::hangup(),
            SignalKind::interrupt(),
            SignalKind::terminate(),
        ];
        for kind in signals_to_handle {
            let mut stream = signal(kind).unwrap();
            let shard_manager = shard_manager.clone();
            tokio::spawn(async move {
                stream.recv().await;
                info!("Signal received - shutting down!");
                shard_manager.lock().await.shutdown_all().await;
            });
        }
    }

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
