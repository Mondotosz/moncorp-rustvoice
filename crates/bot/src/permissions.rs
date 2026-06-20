use poise::serenity_prelude::{self as serenity, Permissions};

#[derive(Copy, Clone)]
pub enum Category {
    Core,
    Privacy,
}

#[derive(Copy, Clone)]
pub struct PermissionEntry {
    pub permission: Permissions,
    pub name: &'static str,
    pub description: &'static str,
    pub category: Category,
}

const VIEW_CHANNEL_ENTRY: PermissionEntry = PermissionEntry {
    permission: Permissions::VIEW_CHANNEL,
    name: "View Channel",
    description: "See channels and guild structure",
    category: Category::Core,
};

const MANAGE_CHANNELS_ENTRY: PermissionEntry = PermissionEntry {
    permission: Permissions::MANAGE_CHANNELS,
    name: "Manage Channels",
    description: "Create, delete, rename, and edit voice channels",
    category: Category::Core,
};

const MOVE_MEMBERS_ENTRY: PermissionEntry = PermissionEntry {
    permission: Permissions::MOVE_MEMBERS,
    name: "Move Members",
    description: "Move users from the primary channel to their temp channel",
    category: Category::Core,
};

const SEND_MESSAGES_ENTRY: PermissionEntry = PermissionEntry {
    permission: Permissions::SEND_MESSAGES,
    name: "Send Messages",
    description: "Post join-request button messages in voice text areas",
    category: Category::Core,
};

const MANAGE_ROLES_ENTRY: PermissionEntry = PermissionEntry {
    permission: Permissions::MANAGE_ROLES,
    name: "Manage Roles",
    // Discord requires this bit to edit channel permission overwrites (PUT /channels/{id}/permissions/{id}).
    // The bot does not create or modify server roles — this permission is used solely for
    // voice channel overwrite management (/private, /public).
    // As an alternative to granting this server-wide, admins can grant "Manage Permissions"
    // (same bit) as a channel-level overwrite on the voice category.
    description: "Edit channel permission overwrites (/private, /public)",
    category: Category::Privacy,
};

/// All tracked permissions with metadata, in display order.
pub const ENTRIES: &[PermissionEntry] = &[
    VIEW_CHANNEL_ENTRY,
    MANAGE_CHANNELS_ENTRY,
    MOVE_MEMBERS_ENTRY,
    SEND_MESSAGES_ENTRY,
    MANAGE_ROLES_ENTRY,
];

/// Permissions required for the bot's core operation.
pub const CORE: Permissions = Permissions::VIEW_CHANNEL
    .union(Permissions::MANAGE_CHANNELS)
    .union(Permissions::MOVE_MEMBERS)
    .union(Permissions::SEND_MESSAGES);

/// Permissions required for privacy features (/private, /public).
/// Can be granted server-wide or at the voice category level (same permission bit).
pub const PRIVACY: Permissions = Permissions::MANAGE_ROLES;

/// Union of all permissions the bot uses; drives the invite URL.
pub const ALL: Permissions = CORE.union(PRIVACY);

fn format_names(required: &[Permissions]) -> String {
    required
        .iter()
        .filter_map(|p| ENTRIES.iter().find(|e| e.permission == *p).map(|e| e.name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A Discord API call failed, and these permissions were required for it.
///
/// `source` is boxed because `serenity::Error` is a large enum.
#[derive(Debug, thiserror::Error)]
#[error("bot permission error (requires: {required_names}): {source}")]
pub struct BotPermissionError {
    pub required: &'static [Permissions],
    required_names: String,
    #[source]
    pub source: Box<serenity::Error>,
}

/// Extension for `Result<T, serenity::Error>`: annotates the error with the
/// permissions the failing operation required.
pub trait PermissionResultExt<T> {
    fn requires(self, required: &'static [Permissions]) -> Result<T, BotPermissionError>;
}

impl<T> PermissionResultExt<T> for Result<T, serenity::Error> {
    fn requires(self, required: &'static [Permissions]) -> Result<T, BotPermissionError> {
        self.map_err(|source| BotPermissionError {
            required,
            required_names: format_names(required),
            source: Box::new(source),
        })
    }
}
