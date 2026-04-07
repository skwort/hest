use directories::ProjectDirs;

use serde::Deserialize;

#[derive(Deserialize)] // Source generation attribute: "Derive" the impl.
pub struct Config {
    #[serde(default)]
    pub handler: HandlerConfig,
    pub transport: TransportConfig,
}

#[derive(Deserialize, Default)]
pub struct HandlerConfig {
    pub reminder: ReminderHandlerConfig,
}

#[derive(Deserialize, Default)]
pub struct ReminderHandlerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub data_dir: Option<String>,
}

#[derive(Deserialize)]
pub struct TransportConfig {
    pub xmpp: XmppConfig,
}

#[derive(Deserialize)]
pub struct XmppConfig {
    pub jid: String,
    pub nick: String,
    pub password_file: String,
    pub rooms: Option<Vec<String>>,
    pub room_status: Option<String>,
}

pub fn load() -> Result<Config, String> {
    let dirs = ProjectDirs::from("dev", "skwort", "hest").unwrap();
    let config_path = dirs.config_dir().join("config.toml");

    let contents = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("{}: {}", config_path.display(), e))?;

    // Return Config if there is no error, else error String
    let mut config: Config = toml::from_str(&contents).map_err(|e| e.to_string())?;
    set_defaults(&dirs, &mut config);
    Ok(config)
}

fn set_defaults(dirs: &ProjectDirs, config: &mut Config) {
    // Set reminder config defaults.
    //
    // An interesting thing here is the use of the get_or_insert* functions.
    // get_or_insert leaves the value unchanged if Some, or sets it to the
    // provided value if None. The _with variant takes a closure.

    let reminder = &mut config.handler.reminder;
    reminder.data_dir.get_or_insert_with(|| {
        dirs.data_dir()
            .join("reminder")
            .to_string_lossy()
            .into_owned()
    });
}

pub fn resolve_password(config: &Config) -> Result<String, String> {
    // Rustisms...
    //
    // read_to_string returns a String, which is heap allocated.
    // We want to trim any trailing newlines, e.g. whitespace. We have two
    // options. The first, and simpler, is to double allocate:
    //
    // std::fs::read_to_string(&config.transport.xmpp.password_file)
    //   .map(|s| s.trim().to_string())
    //
    // In this instance, we allocate once for the return of read_to_string.
    // Trim returns a view into the string, i.e. &str, so we need to clone
    // that (allocate) to match our String return type. The return of read_to_string
    // then gets dropped when we exit the function scope... wasteful.
    //
    // The alternative is cleaner; just mutate in place. Still, feels bit weird
    // compared to how you'd do something like this in C; statically allocated
    // array, write a 0 to the end...
    std::fs::read_to_string(&config.transport.xmpp.password_file)
        .map(|mut s| {
            s.truncate(s.trim_end().len());
            s
        })
        .map_err(|e| format!("{}: {}", &config.transport.xmpp.password_file, e))
}

fn default_true() -> bool {
    true
}
