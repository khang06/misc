use std::{net::IpAddr, sync::Arc, time::Duration};

use serenity::{
    all::{
        ActivityData, ChannelId, CreateButton, CreateInteractionResponse,
        CreateInteractionResponseMessage, CreateMessage, MessageBuilder, ReactionType, Ready,
        UserId,
    },
    async_trait,
    prelude::*,
};

use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{Config, server, verifier::Verifier};

pub struct DiscordVerifier {
    pub config: Arc<Config>,
    pub ctx: Context,
}

impl Verifier for DiscordVerifier {
    async fn ask_for_approval(
        &self,
        req_id: Uuid,
        ip: IpAddr,
        commit: &str,
        size: usize,
    ) -> Option<bool> {
        info!("Asking for approval for request {req_id} ({commit}) from {ip}");

        let content = MessageBuilder::new()
            .mention(&UserId::new(self.config.owner_id))
            .push_line(" HEY!!!!!! NEW CI UPLOAD!!!!!")
            .push_bold("Request ID: ")
            .push_line_safe(req_id)
            .push_bold("Reported commit: ")
            .push_line_safe(commit)
            .push_bold("Requested from: ")
            .push_line(ip.to_string())
            .push_bold("File size: ")
            .push((size / (1024 * 1024)).to_string())
            .push_line(" MB")
            .build();

        let message = ChannelId::new(self.config.approval_channel)
            .send_message(
                &self.ctx,
                CreateMessage::new()
                    .content(&content)
                    .button(
                        CreateButton::new("approved")
                            .emoji(ReactionType::Unicode("✅".to_owned()))
                            .label("Approve"),
                    )
                    .button(
                        CreateButton::new("denied")
                            .emoji(ReactionType::Unicode("❌".to_owned()))
                            .label("Deny"),
                    ),
            )
            .await;

        let Ok(message) = message else {
            error!("Error sending approval request");
            return None;
        };

        let interaction = match message
            .await_component_interaction(&self.ctx.shard)
            .timeout(Duration::from_secs(5 * 60))
            .await
        {
            Some(x) => x,
            None => {
                warn!("No interaction received for commit {commit}");
                message.reply(&self.ctx, "too slow bruh").await.ok();
                return None;
            }
        };

        let (response, result) = match interaction.data.custom_id.as_str() {
            "approved" => ("ok doing it".to_owned(), Some(true)),
            "denied" => ("build denied!!!".to_owned(), Some(false)),
            x => (format!("wtf does {x} mean"), None),
        };
        interaction
            .create_response(
                &self.ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::default().content(response),
                ),
            )
            .await
            .ok();

        info!(
            "Approval result for {req_id} from {ip} was {:?} from {:?}",
            result,
            interaction
                .member
                .map(|x| x.user)
                .map(|x| (x.name, x.id.get()))
        );
        result
    }

    async fn report_error(&self, msg: &str) {
        let content = MessageBuilder::new()
            .mention(&UserId::new(self.config.owner_id))
            .push_bold("OH SHIT!!!!! ")
            .push_codeblock_safe(msg, None)
            .build();

        let message = ChannelId::new(self.config.approval_channel)
            .say(&self.ctx, &content)
            .await;

        if let Err(why) = message {
            error!("Error sending error report: {why:?}");
        };
    }

    async fn report_success(&self, archive_name: &str) {
        let message = ChannelId::new(self.config.approval_channel)
            .say(&self.ctx, "done!")
            .await;
        if let Err(why) = message {
            error!("Error sending approval success: {why:?}");
        };

        let content = MessageBuilder::new()
            .push("new build! ")
            .push(&self.config.base_url)
            .push("/")
            .push_safe(archive_name)
            .build();
        let message = ChannelId::new(self.config.updates_channel)
            .say(&self.ctx, &content)
            .await;
        if let Err(why) = message {
            error!("Error sending update message: {why:?}");
        };
    }
}

#[derive(Clone)]
struct Handler {
    pub config: Arc<Config>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        if let Some(activity) = &self.config.game_activity {
            ctx.set_activity(Some(ActivityData::playing(activity)));
        }

        let message = ChannelId::new(self.config.approval_channel)
            .say(&ctx.http, "hello!!!")
            .await;
        if let Err(why) = message {
            error!("Error sending hello: {why:?}");
        };

        tokio::spawn(server::run(
            Arc::clone(&self.config),
            DiscordVerifier {
                config: Arc::clone(&self.config),
                ctx,
            },
        ));
    }
}

pub async fn run_bot(config: Arc<Config>) {
    let mut client = Client::builder(&config.discord_token, GatewayIntents::empty())
        .event_handler(Handler { config })
        .await
        .expect("Err creating client");

    client.start().await.expect("Failed to start client");
}
