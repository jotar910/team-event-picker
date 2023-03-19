/// The configuration parameters for the application.
#[derive(clap::Parser, Clone)]
pub struct Config {
    /// The connection URL for the database this application should use.
    #[clap(long, env)]
    pub database_url: String,

    /// The name for the database this application should use.
    #[clap(long, env)]
    pub database_name: String,

    /// The signature of the slack workspace that uses this application.
    #[clap(long, env)]
    pub signature: String,

    /// The token for the bot actions in the slack workspace that uses this application.
    #[clap(long, env)]
    pub bot_token: String,

    /// The client id registered for the app slack.
    #[clap(long, env)]
    pub client_id: String,

    /// The client secret registered for the app slack.
    #[clap(long, env)]
    pub client_secret: String,

    /// The PORT number for the server address.
    #[clap(long, env)]
    pub port: u16,
}
