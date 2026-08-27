use serenity::builder::{CreateAttachment, CreateCommand, CreateMessage};
use serenity::http::Http;
use serenity::model::id::{ApplicationId, ChannelId, GuildId, MessageId, RoleId, UserId};
use shared::{ChatMessage, ReactionCount};

pub struct DiscordClient {
    http: Http,
}

impl DiscordClient {
    pub fn new(bot_token: &str, application_id: u64) -> Self {
        let http = Http::new(bot_token);
        http.set_application_id(ApplicationId::new(application_id));
        DiscordClient { http }
    }

    pub async fn has_role(&self, guild_id: u64, user_id: u64, role_id: u64) -> anyhow::Result<bool> {
        match self.http.get_member(GuildId::new(guild_id), UserId::new(user_id)).await {
            Ok(member) => Ok(member.roles.contains(&RoleId::new(role_id))),
            Err(_) => Ok(false), // not a member (404) or any other lookup failure -> treat as not eligible
        }
    }

    pub async fn recent_messages(&self, channel_id: u64, limit: u8) -> anyhow::Result<Vec<ChatMessage>> {
        let messages = self.http.get_messages(ChannelId::new(channel_id), None, Some(limit)).await?;
        Ok(messages
            .into_iter()
            .map(|m| ChatMessage {
                author: m.author.name,
                content: m.content,
                timestamp: m.timestamp.to_string(),
            })
            .collect())
    }

    pub async fn reactions_for_message(&self, channel_id: u64, message_id: u64) -> anyhow::Result<Vec<ReactionCount>> {
        let message = self.http.get_message(ChannelId::new(channel_id), MessageId::new(message_id)).await?;
        Ok(message
            .reactions
            .into_iter()
            .map(|r| ReactionCount { emoji: r.reaction_type.to_string(), count: r.count })
            .collect())
    }

    pub async fn post_candidate(
        &self,
        channel_id: u64,
        png_bytes: Vec<u8>,
        svg_bytes: Vec<u8>,
        short_name: &str,
        caption: &str,
    ) -> anyhow::Result<String> {
        let message = ChannelId::new(channel_id)
            .send_message(
                &self.http,
                CreateMessage::new()
                    .content(caption)
                    .add_file(CreateAttachment::bytes(png_bytes, format!("{short_name}.png")))
                    .add_file(CreateAttachment::bytes(svg_bytes, format!("{short_name}.svg"))),
            )
            .await?;
        Ok(message.id.to_string())
    }

    pub async fn set_guild_commands(&self, guild_id: u64, commands: Vec<CreateCommand>) -> anyhow::Result<()> {
        GuildId::new(guild_id).set_commands(&self.http, commands).await?;
        Ok(())
    }
}
