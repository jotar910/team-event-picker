/// The configuration parameters for the application.
#[derive(clap::Parser, Clone)]
pub struct Config {
    /// The connection URL for the database this application should use.
    #[clap(long, env)]
    pub database_tool_url: String,

    /// The name for the database this application should use.
    #[clap(long, env)]
    pub database_tool_name: String,

    /// The connection URL for the auth database this application should use.
    #[clap(long, env)]
    pub database_auth_url: String,

    /// The name for the auth database this application should use.
    #[clap(long, env)]
    pub database_auth_name: String,

    /// The signature of the slack workspace that uses this application.
    #[clap(long, env)]
    pub signature: String,

    /// The app id registered for the app slack.
    #[clap(long, env)]
    pub app_id: String,

    /// The client id registered for the app slack.
    #[clap(long, env)]
    pub client_id: String,

    /// The client secret registered for the app slack.
    #[clap(long, env)]
    pub client_secret: String,

    /// The PORT number for the server address.
    #[clap(long, env)]
    pub port: u16,

    /// The maximum number of events allowed per channel.
    #[clap(long, env)]
    pub max_events: u32,
}
