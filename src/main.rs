use anyhow::Result;
use clap::Parser;
use log::LevelFilter;
use team_event_picker::config::Config;
use team_event_picker::slack;

#[tokio::main]
async fn main() -> Result<()> {
    // This returns an error if the `.env` file doesn't exist, but that's not what we want
    // since we're not going to use a `.env` file if we deploy this application.
    dotenv::dotenv()?;

    // Initialize the logger.
    env_logger::init();
    log::set_max_level(LevelFilter::Trace);

    // Parse our configuration from the environment.
    // This will exit with a help message if something is wrong.
    let config = Config::parse();

    // We spin up our API.
    slack::serve(config).await?;

    Ok(())
}
