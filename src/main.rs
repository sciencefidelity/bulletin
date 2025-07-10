use bulletin::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let (listener, router) = run("127.0.0.1:8000")?;
    listener.set_nonblocking(true)?;
    let listener = tokio::net::TcpListener::from_std(listener)?;
    axum::serve(listener, router).await
}
