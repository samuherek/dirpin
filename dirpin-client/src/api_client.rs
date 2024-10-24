use dirpin_common::api::HealthCheckResponse;
use eyre::{bail, Result};
use reqwest::{Response, StatusCode};

async fn handle_response_error(res: Response) -> Result<Response> {
    let status = res.status();
    if status == StatusCode::SERVICE_UNAVAILABLE {
        bail!("Service unavailable.");
    }

    if !status.is_success() {
        // TODO: account for all the cases
        bail!("There was an error with the service: Status {status:?}.");
    }

    Ok(res)
}

pub async fn health_check(address: &str) -> Result<HealthCheckResponse> {
    let url = format!("{address}/");
    let res = reqwest::get(url).await?;
    let res = handle_response_error(res).await?;

    let res = res.json::<HealthCheckResponse>().await?;
    Ok(res)
}
