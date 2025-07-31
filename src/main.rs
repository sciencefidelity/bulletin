// use std::io::IsTerminal;

use bulletin::telemetry::{Formatter, get_subscriber, init_subscriber};
use bulletin::{Application, configuration};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // color_eyre::config::HookBuilder::default()
    //     .theme(if !std::io::stderr().is_terminal() {
    //         color_eyre::config::Theme::new()
    //     } else {
    //         color_eyre::config::Theme::dark()
    //     })
    //     .install()
    //     .unwrap();

    const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

    let subscriber = get_subscriber(
        PACKAGE_NAME.into(),
        "info".into(),
        &Formatter::Bunyan,
        std::io::stdout,
    );
    init_subscriber(subscriber);

    let configuration = configuration::get().expect("failed to read configuration");
    let application = Application::build(configuration)?;
    application.run_until_stopped().await
}
