use dirpin_common::api::{
    AddEntryRequest, HealthCheckResponse, LoginRequest, LoginResponse, LogoutResponse,
    RegisterRequest, RegisterResponse, SyncResponse,
};
use eyre::{bail, Result};
use reqwest::header::{HeaderMap, AUTHORIZATION};
use reqwest::{Response, StatusCode};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub struct AuthClient<'a> {
    address: &'a str,
    client: reqwest::Client,
}

impl<'a> AuthClient<'a> {
    pub fn new(address: &'a str, session_token: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Token {}", session_token).parse()?);
        // TODO add user agents;
        // TODO: add version header;
        // TODO: connection timeout
        // TODO: timeout

        Ok(Self {
            address,
            client: reqwest::Client::builder()
                // .user_agent()
                .default_headers(headers)
                .build()?,
        })
    }

    pub async fn logout(&self) -> Result<LogoutResponse> {
        let url = format!("{}/logout", self.address);
        let res = self.client.get(url).send().await?;
        let res = handle_response_error(res).await?;
        let res = res.json::<LogoutResponse>().await?;

        Ok(res)
    }

    pub async fn sync(&self, from: OffsetDateTime) -> Result<SyncResponse> {
        let url = format!(
            "{}/sync?last_sync_ts={}",
            self.address,
            urlencoding::encode(from.format(&Rfc3339)?.as_str())
        );
        let res = self.client.get(url).send().await?;
        let res = handle_response_error(res).await?;
        let res = res.json::<SyncResponse>().await?;

        Ok(res)
    }

    pub async fn post_entries(&self, data: &[AddEntryRequest]) -> Result<()> {
        let url = format!("{}/entries", self.address);
        let res = self.client.post(url).json(data).send().await?;
        handle_response_error(res).await?;

        Ok(())
    }
}

async fn handle_response_error(res: Response) -> Result<Response> {
    let status = res.status();
    if status == StatusCode::SERVICE_UNAVAILABLE {
        bail!("Service unavailable.");
    }

    if !status.is_success() {
        println!("{res:?}");
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

pub async fn register(
    address: &str,
    username: &str,
    email: &str,
    password: &str,
    host_id: &str,
) -> Result<RegisterResponse> {
    // TODO: check if the user already exists
    // TODO: setup the headers with version

    let url = format!("{address}/register");
    let res = reqwest::Client::new()
        .post(url)
        .json(&RegisterRequest {
            username: username.into(),
            email: email.into(),
            password: password.into(),
            host_id: host_id.into(),
        })
        .send()
        .await?;
    let res = handle_response_error(res).await?;
    let res = res.json::<RegisterResponse>().await?;

    Ok(res)
}

// TODO: make sure the passwords are behind "secretbox"
pub async fn login(
    address: &str,
    username: &str,
    password: &str,
    host_id: &str,
) -> Result<LoginResponse> {
    let url = format!("{address}/login");
    let res = reqwest::Client::new()
        .post(url)
        .json(&LoginRequest {
            username: username.into(),
            password: password.into(),
            host_id: host_id.into(),
        })
        .send()
        .await?;
    let res = handle_response_error(res).await?;
    let res = res.json::<LoginResponse>().await?;

    Ok(res)
}
