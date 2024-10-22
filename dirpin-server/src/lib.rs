use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{transport::Server, Request, Response, Status};

use dirpin_common::echo::echo_server::{Echo, EchoServer};
use dirpin_common::echo::{EchoRequest, EchoResponse};

#[derive(Debug, Default)]
pub struct EchoService {}

#[tonic::async_trait]
impl Echo for EchoService {
    async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoResponse>, Status> {
        let message = request.into_inner().message;
        println!("message: {}", message);
        let reply = EchoResponse { message };
        Ok(Response::new(reply))
    }
}

pub async fn start_server() -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = "/tmp/echo.sock";

    if std::path::Path::new(socket_path).exists() {
        tokio::fs::remove_file(socket_path).await?;
    }

    let uds = UnixListener::bind(socket_path)?;
    let uds_stream = UnixListenerStream::new(uds);

    println!("Server listening on {}", socket_path);

    Server::builder()
        .add_service(EchoServer::new(EchoService::default()))
        .serve_with_incoming(uds_stream)
        .await?;

    Ok(())
}
