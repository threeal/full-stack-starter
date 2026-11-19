//! Runs the click counter app, serving its embedded web frontend.

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    println!("Server listening at http://{}", listener.local_addr()?);
    axum::serve(listener, counter::router()).await
}
