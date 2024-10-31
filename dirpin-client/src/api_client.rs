use dirpin_common::api::{AddPinRequest, HealthCheckResponse, SyncRequest, SyncResponse};
use eyre::{bail, Result};
use reqwest::{Response, StatusCode};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

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

pub async fn handle_sync(address: &str, from: OffsetDateTime) -> Result<SyncResponse> {
    let url = format!(
        "{address}/sync?last_sync_ts={}",
        urlencoding::encode(from.format(&Rfc3339)?.as_str())
    );
    let res = reqwest::get(url).await?;
    let res = handle_response_error(res).await?;
    let res = res.json::<SyncResponse>().await?;

    Ok(res)
}

pub async fn handle_post_pins(address: &str, data: &[AddPinRequest]) -> Result<()> {
    let url = format!("{address}/pins");
    let res = reqwest::Client::new().post(url).json(data).send().await?;
    handle_response_error(res).await?;

    Ok(())
}
