use spade::app::App;
use spade::config::Config;
use spade::config_file::load_config;

#[tokio::main(flavor = "current_thread")]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Config::parse();
    let file_config = load_config();

    if cli.prototype() {
        return spade::screens::design_prototype::run_prototype().await;
    }

    let mut app = App::new(cli, file_config);
    app.run().await
}
