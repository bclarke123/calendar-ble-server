use std::time::Duration;

use serde::Serialize;
use tokio::sync::watch::Sender;
use tokio::time;

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

impl<'a> CalendarInfo<'a> {
    pub async fn fetch_current_status() -> anyhow::Result<Self> {
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
