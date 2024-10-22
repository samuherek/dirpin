use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn; // Add this import

use dirpin_common::echo::echo_client::EchoClient;
use dirpin_common::echo::EchoRequest;

pub async fn start_client() -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = "/tmp/echo.sock";
    let message = std::env::args()
        .nth(1)
        .unwrap_or("Hello, world!".to_string());

    let endpoint = Endpoint::try_from("http://[::]:50051")?;

    let channel = endpoint
        .connect_with_connector(service_fn(move |_: Uri| {
            let path = socket_path.to_owned();
            async move {
                // Connect to the Unix socket
                let stream = UnixStream::connect(path).await?;
                // Wrap it with TokioIo to implement the required traits
                Ok::<_, std::io::Error>(TokioIo::new(stream))
            }
        }))
        .await?;

    let mut client = EchoClient::new(channel);

    let request = tonic::Request::new(EchoRequest { message });

    let response = client.echo(request).await?;

    println!("Response: {:?}", response.into_inner().message);

    Ok(())
}








