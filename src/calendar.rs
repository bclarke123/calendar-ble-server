use std::time::Duration;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum Status {
    Busy,
    Free,
    Focus,
}

#[derive(Debug, Serialize)]
pub struct CalendarInfo {
    status: Status,
    start_time: [u8; 2],
    duration: u8,
    label: String,
}

impl CalendarInfo {
    pub async fn fetch_current_status() -> anyhow::Result<Self> {
        // TODO make an HTTP request when I'm not on a plane

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
}
