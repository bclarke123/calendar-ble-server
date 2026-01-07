use std::time::Duration;

use btleplug::api::{Central as _, Manager as _, Peripheral, ScanFilter};
use btleplug::api::{CentralEvent, WriteType};
use btleplug::platform::Manager;
use chrono::Local;
use futures::StreamExt;
use serde::Serialize;
use tokio::sync::watch;
use tokio::time::{self, Instant};
use uuid::Uuid;

use crate::calendar::CalendarInfo;

const TARGET_DEVICE: &str = "DoorSign";
const TARGET_CHAR_UUID_STR: &str = "9ecadc24-0ee5-a9e0-93f3-a3b50200406e";
const COOLDOWN_SEC: u64 = 30;

#[derive(Debug, Serialize)]
pub struct DisplayRequest {
    current_time: String,
    padding: String,
    calendar_info: CalendarInfo,
}

pub async fn run(rx: watch::Receiver<Option<CalendarInfo>>) -> ! {
    let char_uuid = uuid::Uuid::parse_str(TARGET_CHAR_UUID_STR).expect("Invalid UUID");

    let manager = Manager::new()
        .await
        .expect("Couldn't create bluetooth manager");
    let adapters = manager
        .adapters()
        .await
        .expect("Couldn't get bluetooth adapters");
    let adapter = adapters.first().expect("No bluetooth adapter found");

    let filter = ScanFilter::default();
    // filter.services.push(uuid);

    adapter
        .start_scan(filter.clone())
        .await
        .expect("Couldn't start bluetooth scan");
    let mut last_update = Instant::now() - Duration::from_secs(COOLDOWN_SEC);
    let mut events = adapter
        .events()
        .await
        .expect("Couldn't get bluetooth events");

    println!("Bluetooth listener running");

    while let Some(event) = events.next().await {
        match event {
            CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => {
                let peripheral = adapter
                    .peripheral(&id)
                    .await
                    .expect("Couldn't get peripheral");
                let props = match peripheral
                    .properties()
                    .await
                    .expect("Couldn't get peripheral properties")
                {
                    Some(props) => props,
                    None => continue,
                };

                let local_name = props.local_name.unwrap_or_default();

                if !local_name.contains(TARGET_DEVICE) {
                    continue;
                }

                // println!("Event from {}", local_name);

                if Instant::now() - last_update < Duration::from_secs(COOLDOWN_SEC) {
                    continue;
                }

                println!("Got wakeup call, sending latest data");

                let str_data = match &*rx.borrow() {
                    Some(obj) => {
                        let now = Local::now();
                        let req = DisplayRequest {
                            current_time: now.format("%Y-%m-%dT%H:%M:%S%.6f%:z").to_string(),
                            padding: "          ".to_string(),
                            calendar_info: obj.clone(),
                        };

                        serde_json::to_string(&req).unwrap_or_default()
                    }
                    None => continue,
                };

                // println!("Serialized data {}", &str_data);

                adapter.stop_scan().await.ok();

                if let Err(e) = write_data(&peripheral, str_data.as_bytes(), char_uuid).await {
                    println!("Error writing calendar data: {}", e);
                } else {
                    println!("Wrote calendar data!");
                    last_update = Instant::now();
                }

                adapter
                    .start_scan(filter.clone())
                    .await
                    .expect("Couldn't start bluetooth scan");
            }
            _ => {}
        }
    }

    unreachable!();
}

async fn write_data<P: Peripheral>(peripheral: &P, data: &[u8], uuid: Uuid) -> anyhow::Result<()> {
    time::timeout(Duration::from_secs(5), peripheral.connect()).await??;
    peripheral.discover_services().await?;

    let chars = peripheral.characteristics();

    let target_char = match chars.iter().find(|x| x.uuid == uuid) {
        Some(char) => char,
        None => anyhow::bail!("Couldn't find characteristic on target device: {}", uuid),
    };

    println!("Writing status...");

    peripheral
        .write(target_char, data, WriteType::WithResponse)
        .await?;

    peripheral.disconnect().await?;

    Ok(())
}
