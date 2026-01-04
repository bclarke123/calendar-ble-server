use std::{sync::LazyLock, time::Duration};

use reqwest::Client;
use serde::Serialize;
use tokio::{sync::watch::Sender, time};

#[derive(Copy, Clone, Debug, Serialize)]
pub enum Status {
    Busy,
    // Free,
    // Focus,
}

#[derive(Copy, Clone, Debug, Serialize)]
pub struct CalendarInfo<'a> {
    status: Status,
    start_time: [u8; 2],
    duration: u8,
    label: &'a str,
}

const TOKEN_OBJ: &str = include_str!("../token.json");

impl<'a> CalendarInfo<'a> {
    pub async fn fetch_current_status() -> anyhow::Result<Self> {
        static CLIENT: LazyLock<Client> = LazyLock::new(Client::new);
        static TOKEN: LazyLock<String> = LazyLock::new(|| {
            serde_json::from_str::<serde_json::Value>(TOKEN_OBJ)
                .ok()
                .and_then(|v| v["token"].as_str().map(|s| s.to_string()))
                .unwrap()
        });

        let now = chrono::Utc::now().to_rfc3339();

        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/primary/events?timeMin={}&maxResults=1&singleEvents=true&orderBy=startTime",
            now
        );

        let resp = CLIENT.get(url).bearer_auth(&*TOKEN).send().await;

        // TODO make an HTTP request when I'm not on a plane

        Ok(CalendarInfo {
            status: Status::Busy,
            start_time: [17, 0],
            duration: Self::get_duration(Duration::from_mins(45)),
            label: "Important Stuff",
        })
    }

    fn get_duration(duration: Duration) -> u8 {
        ((duration.as_secs() / 60) / 5).clamp(0, 255) as u8
    }
}

pub async fn sync_task<'a>(tx: Sender<Option<CalendarInfo<'a>>>) -> ! {
    loop {
        if let Ok(latest) = CalendarInfo::fetch_current_status().await {
            tx.send(Some(latest)).ok();
        }

        time::sleep(Duration::from_secs(60)).await;
    }
}
