use crate::discord::DiscordClient;
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::CommandOptionType;
use serenity::model::Permissions;

pub async fn register(discord: &DiscordClient, guild_id: u64) -> anyhow::Result<()> {
    let token_command = CreateCommand::new("token")
        .description("Get your personal worker token (DM only, requires the logo-team role)");

    let revoke_command = CreateCommand::new("revoke")
        .description("Revoke a team member's worker token")
        .default_member_permissions(Permissions::MANAGE_GUILD)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "The team member to revoke")
                .required(true),
        );

    discord.set_guild_commands(guild_id, vec![token_command, revoke_command]).await
}
