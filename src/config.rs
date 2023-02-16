/// The configuration parameters for the application.
#[derive(clap::Parser)]
pub struct Config {
    /// The connection URL for the database this application should use.
    #[clap(long, env)]
    pub database_url: String,

    /// The name for the database this application should use.
    #[clap(long, env)]
    pub database_name: String,

    /// The PORT number for the server address.
    #[clap(long, env)]
    pub port: u16,
}
