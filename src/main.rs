use bulletin::telemetry::{Formatter, get_subscriber, init_subscriber};
use bulletin::{configuration, run};
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

    let subscriber = get_subscriber(
        PACKAGE_NAME.into(),
        "info".into(),
        &Formatter::Bunyan,
        std::io::stdout,
    );
    init_subscriber(subscriber);

    let configuration = configuration::get().expect("failed to read configuration");
    let connection_pool = PgPool::connect_lazy_with(configuration.database.with_db());
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let (listener, router) = run(address.as_str(), connection_pool)?;
    listener.set_nonblocking(true)?;
    let listener = tokio::net::TcpListener::from_std(listener)?;
    axum::serve(listener, router).await
}
