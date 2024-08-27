use anyhow::Result;
use clap::Parser;
use log::LevelFilter;
use team_event_picker::config::Config;
use team_event_picker::slack;

#[tokio::main]
async fn main() -> Result<()> {
    // This returns an error if the `.env` file doesn't exist, but that's not what we want
    // since we're not going to use a `.env` file if we deploy this application.
    let dotenv_result = dotenv::dotenv();

    // Initialize the logger.
    tracing_subscriber::fmt::init();
    log::set_max_level(LevelFilter::Trace);

    if let Err(err) = dotenv_result {
        log::warn!("could not load .env file: {}", err);
    } else {
        log::info!("loaded .env file");
    };

    // Parse our configuration from the environment.
    // This will exit with a help message if something is wrong.
    let config = Config::parse();

    // We spin up our API.
    slack::serve(config).await?;

    Ok(())
}
