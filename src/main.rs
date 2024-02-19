use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serenity::all::{Member, Message, Permissions};
use serenity::builder::CreateMessage;
use serenity::prelude::{CacheHttp, Context, EventHandler, GatewayIntents, TypeMapKey};
use serenity::{async_trait, Client};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
struct Data {
    channel_id: u64,
    verified_role: u64,
}

struct StringType;

impl TypeMapKey for StringType {
    type Value = String;
}

struct ListMembers;

impl TypeMapKey for ListMembers {
    type Value = HashMap<u64, String>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        if let Some(content) = ctx.data.read().await.get::<StringType>() {
            if let Ok(data) = serde_json::from_str::<Data>(content) {
                let captcha: String = rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(7)
                    .map(char::from)
                    .collect();
                new_member
                    .guild_id
                    .channels(&ctx.http)
                    .await
                    .unwrap()
                    .get(&data.channel_id.into())
                    .unwrap()
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().content(format!(
                            "<@{}> scrivi `!verify {}` per verificarti",
                            new_member.user.id.get(),
                            captcha
                        )),
                    )
                    .await
                    .unwrap();
                ctx.data
                    .write()
                    .await
                    .get_mut::<ListMembers>()
                    .unwrap()
                    .insert(new_member.user.id.get(), captcha);
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content.starts_with("!verify") {
            if let Some(content) = ctx.data.read().await.get::<StringType>() {
                if let Ok(data) = serde_json::from_str::<Data>(content) {
                    if msg.channel_id.get() == data.channel_id {
                        if let Ok(member) = msg.member(ctx.clone()).await {
                            member
                                .add_role(ctx.http(), data.verified_role)
                                .await
                                .expect("can't add role to member");
                        }
                    }
                }
            }
        } else if msg.content.starts_with("!ver-config") {
            if msg
                .guild_id
                .unwrap()
                .member(ctx, msg.author.id)
                .await
                .unwrap()
                .permissions(&ctx)
                .unwrap()
                .contains(Permissions::ADMINISTRATOR)
            {
                let mut split = msg.content.split(' ');
                let channel_id = split.nth(1).unwrap().parse().unwrap();
                let verified_role = split.next().unwrap().parse().unwrap();
                let data = Data {
                    channel_id,
                    verified_role,
                };

                ctx.data
                    .write()
                    .await
                    .insert::<StringType>(serde_json::to_string(&data).unwrap());
            }
        }
    }

    async fn ready(&self, ctx: Context, _data_about_bot: serenity::all::Ready) {
        println!("Starting N Verify");
        if let Ok(file) = fs::read_to_string("saved_data") {
            ctx.data.write().await.insert::<StringType>(file);
        }
    }
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
