//! Holds prefix-command definition structs.

use crate::{serenity_prelude as serenity, BoxFuture, Framework};

/// Prefix-specific context passed to command invocations.
///
/// Contains the trigger message, the Discord connection management stuff, and the user data.
pub struct PrefixContext<'a, U, E> {
    /// Serenity's context, like HTTP or cache
    pub discord: &'a serenity::Context,
    /// The invoking user message
    pub msg: &'a serenity::Message,
    /// Prefix used by the user to invoke this command
    pub prefix: &'a str,
    /// Read-only reference to the framework
    ///
    /// Useful if you need the list of commands, for example for a custom help command
    pub framework: &'a Framework<U, E>,
    /// The command object which is the current command
    ///
    /// Optional to allow using [`PrefixContext`] in places where there is no command. For example
    /// Etternabot's message listener
    pub command: Option<&'a PrefixCommand<U, E>>,
    /// Your custom user data
    pub data: &'a U,
}
// manual Copy+Clone implementations because Rust is getting confused about the type parameter
impl<U, E> Clone for PrefixContext<'_, U, E> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<U, E> Copy for PrefixContext<'_, U, E> {}
impl<U, E> crate::_GetGenerics for PrefixContext<'_, U, E> {
    type U = U;
    type E = E;
}

/// Optional settings for a [`PrefixCommand`].
pub struct PrefixCommandOptions<U, E> {
    /// Multiline description with detailed usage instructions. Displayed in the command specific
    /// help: `~help command_name`
    // TODO: fix the inconsistency that this is String and everywhere else it's &'static str
    pub multiline_help: Option<fn() -> String>,
    /// Alternative triggers for the command
    pub aliases: &'static [&'static str],
    /// Falls back to the framework-specified value on None. See there for documentation.
    pub on_error: Option<fn(E, PrefixCommandErrorContext<'_, U, E>) -> BoxFuture<'_, ()>>,
    /// If this function returns false, this command will not be executed.
    pub check: Option<fn(PrefixContext<'_, U, E>) -> BoxFuture<'_, Result<bool, E>>>,
    /// Whether to enable edit tracking for commands by default.
    ///
    /// Note: only has an effect if `Framework::edit_tracker` is set.
    pub track_edits: bool,
    /// Whether to broadcast a typing indicator while executing this commmand.
    pub broadcast_typing: bool,
}

impl<U, E> Default for PrefixCommandOptions<U, E> {
    fn default() -> Self {
        Self {
            multiline_help: None,
            check: None,
            on_error: None,
            aliases: &[],
            track_edits: false,
            broadcast_typing: false,
        }
    }
}

/// Definition of a single command, excluding metadata which doesn't affect the command itself such
/// as category.
pub struct PrefixCommand<U, E> {
    /// Main name of the command. Aliases can be set in [`PrefixCommandOptions::aliases`].
    pub name: &'static str,
    /// Callback to execute when this command is invoked.
    pub action: for<'a> fn(PrefixContext<'a, U, E>, args: &'a str) -> BoxFuture<'a, Result<(), E>>,
    /// The command ID, shared across all command types that belong to the same implementation
    pub id: std::sync::Arc<crate::CommandId>,
    /// Optional data to change this command's behavior.
    pub options: PrefixCommandOptions<U, E>,
}

/// Includes a command, plus metadata like associated sub-commands or category.
pub struct PrefixCommandMeta<U, E> {
    /// Core command data
    pub command: PrefixCommand<U, E>,
    /// Possible subcommands
    pub subcommands: Vec<PrefixCommandMeta<U, E>>,
}

/// Context passed alongside the error value to error handlers
pub struct PrefixCommandErrorContext<'a, U, E> {
    /// What part of the command triggered the error
    pub location: crate::CommandErrorLocation,
    /// Which command was being processed when the error occured
    pub command: &'a PrefixCommand<U, E>,
    /// Further context
    pub ctx: PrefixContext<'a, U, E>,
}

impl<U, E> Clone for PrefixCommandErrorContext<'_, U, E> {
    fn clone(&self) -> Self {
        Self {
            location: self.location,
            command: self.command,
            ctx: self.ctx,
        }
    }
}

/// Possible ways to define a command prefix
pub enum Prefix {
    /// A case-sensitive string literal prefix (passed to [`str::strip_prefix`])
    Literal(&'static str),
    /// Regular expression which matches the prefix
    Regex(regex::Regex),
}

/// Prefix-specific framework configuration
pub struct PrefixFrameworkOptions<U, E> {
    /// The main bot prefix. Can be set to None if the bot supports only
    /// [dynamic prefixes](Self::dynamic_prefix).
    pub prefix: Option<String>,
    /// List of bot commands.
    pub commands: Vec<PrefixCommandMeta<U, E>>,
    /// List of additional bot prefixes
    // TODO: maybe it would be nicer to have separate fields for literal and regex prefixes
    // That way, you don't need to wrap every single literal prefix in a long path which looks ugly
    pub additional_prefixes: Vec<Prefix>,
    /// Callback invoked on evevry message to return a prefix.
    ///
    /// If Some is returned, the static prefix, along with the additional prefixes will be ignored,
    /// and the returned prefix will be used for checking, but if None is returned, the static
    /// prefix and additional prefixes will be checked instead.
    ///
    /// Override this field for a simple dynamic prefixe which changes depending on the guild or user.
    pub dynamic_prefix: Option<
        for<'a> fn(
            &'a serenity::Context,
            &'a serenity::Message,
            &'a U,
        ) -> BoxFuture<'a, Option<String>>,
    >,
    /// Callback invoked on every message to strip the prefix off an incoming message.
    ///
    /// Override this field for dynamic prefixes which change depending on guild or user.
    ///
    /// Return value is a tuple of the prefix and the rest of the message:
    /// ```rust,ignore
    /// if msg.content.starts_with(my_cool_prefix) {
    ///     return Some(msg.content.split_at(my_cool_prefix.len()));
    /// }
    /// ```
    pub stripped_dynamic_prefix: Option<
        for<'a> fn(
            &'a serenity::Context,
            &'a serenity::Message,
            &'a U,
        ) -> BoxFuture<'a, Option<(&'a str, &'a str)>>,
    >,
    /// Treat a bot mention (a ping) like a prefix
    pub mention_as_prefix: bool,
    /// If Some, the framework will react to message edits by editing the corresponding bot response
    /// with the new result.
    pub edit_tracker: Option<std::sync::RwLock<super::EditTracker>>,
    /// If the user makes a typo in their message and a subsequent edit creates a valid invocation,
    /// the bot will execute the command if this attribute is set. [`Self::edit_tracker`] does not
    /// need to be set for this.
    ///
    /// That does not mean that any subsequent edits will also trigger execution. For that,
    /// see [`PrefixCommandOptions::track_edits`].
    ///
    /// Note: only has an effect if [`Self::edit_tracker`] is set.
    pub execute_untracked_edits: bool,
    /// Wether or not to ignore message edits on messages outside the cache.
    /// This can happen if the message edit happens while the command is being invoked, or the
    /// original message wasn't a command.
    pub ignore_edit_tracker_cache: bool,

    /// Whether commands in messages emitted by the bot itself should be executed as well.
    pub execute_self_messages: bool,
    /// Whether command names should be compared case-insensitively.
    pub case_insensitive_commands: bool,
    /* // TODO: implement
    /// Whether to invoke help command when someone sends a message with just a bot mention
    pub help_when_mentioned: bool,
    /// The bot's general help command. Currently used for [`Self::help_when_mentioned`].
    pub help_commmand: Option<PrefixCommand<U, E>>,
    // /// The bot's help command for individial commands. Currently used when a command group without
    // /// any specific subcommand is invoked. This command is expected to take the command name as a
    // /// single parameter
    // pub command_specific_help_commmand: Option<PrefixCommand<U, E>>, */
}

impl<U, E> Default for PrefixFrameworkOptions<U, E> {
    fn default() -> Self {
        Self {
            prefix: None,
            commands: Vec::new(),
            additional_prefixes: Vec::new(),
            dynamic_prefix: None,
            stripped_dynamic_prefix: None,
            mention_as_prefix: true,
            edit_tracker: None,
            execute_untracked_edits: true,
            ignore_edit_tracker_cache: false,
            execute_self_messages: false,
            case_insensitive_commands: true,
            // help_when_mentioned: true,
            // help_commmand: None,
            // command_specific_help_commmand: None,
        }
    }
}
