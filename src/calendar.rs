use std::{sync::LazyLock, time::Duration};

use anyhow::Context;
use chrono::{DateTime, FixedOffset, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{fs, sync::watch::Sender, time};

const TOKEN_FILE: &str = "token.json";
static CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("Unauthorized")]
    Unauthorized,
}

#[derive(Copy, Clone, Debug, Serialize)]
pub enum Status {
    Busy,
    // Free,
    // Focus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarDate {
    date: Option<NaiveDate>,
    date_time: Option<DateTime<FixedOffset>>,
}

#[derive(Debug, Deserialize)]
struct CalendarItem {
    start: Option<CalendarDate>,
    end: Option<CalendarDate>,
    summary: String,
    eventType: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CalendarResponse {
    items: Vec<CalendarItem>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CalendarInfo {
    status: Status,
    start_time: [u8; 2],
    duration: u8,
    label: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OAuthCredentials {
    pub token: String,
    pub refresh_token: Option<String>,
    pub token_uri: String,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<String>,
}

#[derive(Debug)]
struct CalendarClient {
    creds: OAuthCredentials,
}

impl CalendarClient {
    pub async fn new() -> anyhow::Result<Self> {

        let json = fs::read_to_string(TOKEN_FILE).await
            .context("Couldn't read token.json")?;

        let creds = serde_json::from_str::<OAuthCredentials>(&json)
            .context("Couldn't parse token.json")?;

        Ok(Self { creds })
    }

    pub async fn fetch_current_status(&self) -> anyhow::Result<CalendarInfo> {
        let now_local = Local::now();
        let today_start = now_local.date_naive().and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        let today_end = now_local.date_naive().and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap());
        let time_min = Local.from_local_datetime(&today_start).unwrap().to_rfc3339();
        let time_max = Local.from_local_datetime(&today_end).unwrap().to_rfc3339();

        let params = [
            ("calendarId", "primary"),
            ("singleEvents", "true"),
            ("orderBy", "startTime"),
            ("timeMin", &time_min),
            ("timeMax", &time_max),
        ];

        println!("Sending request for calendar...");

        let resp = CLIENT.get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
            .query(&params)
            .bearer_auth(&self.creds.token)
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            println!("Unauthorized");
            anyhow::bail!(FetchError::Unauthorized);
        }

        println!("Received response");

        let json = resp.json::<CalendarResponse>().await?;

        println!("{:?}", json);

        Ok(CalendarInfo {
            status: Status::Busy,
            start_time: [17, 0],
            duration: Self::get_duration(Duration::from_mins(45)),
            label: "Important Stuff".to_string(),
        })
    }

    fn get_duration(duration: Duration) -> u8 {
        ((duration.as_secs() / 60) / 5).clamp(0, 255) as u8
    }

    async fn refresh_token(&mut self) -> anyhow::Result<()> {
        println!("Attempting to refresh token...");

        let refresh_token = self.creds.refresh_token.as_ref().context("No refresh token")?;

        let params = [
            ("client_id", &self.creds.client_id),
            ("client_secret", &self.creds.client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", &"refresh_token".to_string())
        ];

        let resp = CLIENT.post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Couldn't refresh token: {}", resp.status());
        }

        let data: serde_json::Value = resp.json().await?;

        if let Some(new_token) = data["access_token"].as_str() {
            self.creds.token = new_token.to_string();
            self.save_creds().await?;

            println!("Token refreshed");

            Ok(())
        } else {
            anyhow::bail!("No access token in response");
        }
    }

    async fn save_creds(&self) -> anyhow::Result<()> {
        let json = serde_json::to_string(&self.creds)?;
        fs::write(TOKEN_FILE, json).await?;
        Ok(())
    }
}

pub async fn sync_task(tx: Sender<Option<CalendarInfo>>) -> ! {
    let mut calendar = CalendarClient::new().await.unwrap();

    loop {
        match calendar.fetch_current_status().await {
            Ok(latest) => {
                tx.send(Some(latest)).ok();
            },
            Err(e) => {
                if let Some(err) = e.downcast_ref::<FetchError>() && matches!(err, FetchError::Unauthorized) {
                    calendar.refresh_token().await.ok();
                } else {
                    println!("Error fetching status: {}", e);
                }
            }
        }

        time::sleep(Duration::from_secs(60)).await;
    }
}
