use directories::ProjectDirs;

use serde::Deserialize;

#[derive(Deserialize)] // Source generation attribute: "Derive" the impl.
pub struct Config {
    pub transport: TransportConfig,
}

#[derive(Deserialize)]
pub struct TransportConfig {
    pub xmpp: XmppConfig,
}

#[derive(Deserialize)]
pub struct XmppConfig {
    pub jid: String,
    pub nick: String,
    pub rooms: Option<Vec<String>>,
    pub room_status: Option<String>,
}

pub fn load() -> Result<Config, String> {
    let dirs = ProjectDirs::from("dev", "skwort", "hest").unwrap();
    let config_path = dirs.config_dir().join("config.toml");

    let contents = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("{}: {}", config_path.display(), e))?;

    // Return Config if there is no error, else error String
    toml::from_str(&contents).map_err(|e| e.to_string())
}
