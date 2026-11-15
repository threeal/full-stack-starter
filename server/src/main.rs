//! Runs the server, serving the built app from the directory in `PUBLIC_DIR`.

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let public_dir = std::env::var_os("PUBLIC_DIR").map(std::path::PathBuf::from);
    if public_dir.is_none() {
        eprintln!("PUBLIC_DIR is not set. Static file handling is disabled.");
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    println!("Server listening at http://{}", listener.local_addr()?);
    axum::serve(listener, server::router(public_dir.as_deref())).await
}
