#![feature(allocator_api)]

use std::{error::Error, sync::Arc, time::{SystemTime, Instant}};

use dotenv::dotenv;
use futures::StreamExt;
use std::time::UNIX_EPOCH;
use twilight_gateway::{cluster::ShardScheme, Cluster, Event, Intents};
use twilight_http::Client;
use twilight_model::{
    application::{callback::InteractionResponse, command::CommandType},
    channel::message::MessageFlags,
    id::ApplicationId,
};
use twilight_util::builder::{command::CommandBuilder, CallbackDataBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenv()?;
    let token = std::env::var("BOT_TOKEN")?;
    let scheme = ShardScheme::Auto;

    let (cluster, mut events) = Cluster::builder(token.to_owned(), Intents::empty())
        .shard_scheme(scheme)
        .build()
        .await?;
    let cluster = Arc::new(cluster);

    let cluster_spawn = Arc::clone(&cluster);

    tokio::spawn(async move {
        cluster_spawn.up().await;
    });

    let command = CommandBuilder::new(
        "ping".into(),
        String::from("Check the API latency."),
        CommandType::ChatInput,
    )
    .build();

    let client = Arc::new(Client::new(token));
    client.set_application_id(ApplicationId::new(927552868117012531).unwrap());
    client.set_global_commands(&[command])?.exec().await?;

    while let Some((shard_id, event)) = events.next().await {
        tokio::spawn(handle_event(shard_id, event, Arc::clone(&client)));
    }

    Ok(())
}

async fn handle_event(
    shard_id: u64,
    event: Event,
    client: Arc<Client>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match event {
        Event::InteractionCreate(interaction) => match interaction.0 {
            twilight_model::application::interaction::Interaction::ApplicationCommand(cmd) => {
                match &*cmd.data.name {
                    "ping" => {
                        let now = Instant::now();
                        let sys_time = SystemTime::now();
                        let cmd_sent = ((cmd.id.get() >> 22) + 1420070400000) as u128;
                        let ping_time = sys_time.duration_since(UNIX_EPOCH)?.as_millis() - cmd_sent;

                        let res = InteractionResponse::DeferredChannelMessageWithSource(
                            CallbackDataBuilder::new()
                                .flags(MessageFlags::EPHEMERAL)
                                .build(),
                        );
                        client
                            .interaction_callback(cmd.id, &cmd.token, &res)
                            .exec()
                            .await?;

                        let elapsed = now.elapsed().as_millis();

                        dbg!(&cmd_sent, &ping_time, &elapsed);

                        client
                            .update_interaction_original(&cmd.token)?
                            .content(Some(&*format!(
                                "Pong in {}ms\nRoundtrip in {}ms",
                                ping_time, elapsed
                            )))?
                            .exec()
                            .await?;
                    }
                    _ => (),
                }
            }
            _ => (),
        },
        Event::ShardConnected(_) => {
            println!("Connected on shard {}", shard_id);
        }
        _ => {}
    }

    Ok(())
}
