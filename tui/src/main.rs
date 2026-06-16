use spade::app::App;
use spade::config::Config;
use spade::config_file::load_config;

#[tokio::main(flavor = "current_thread")]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let cli = Config::parse();
    let file_config = load_config();

    let mut app = App::new(cli, file_config);
    app.run().await
}
