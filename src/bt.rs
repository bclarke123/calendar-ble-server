use std::time::Duration;

use btleplug::api::{CentralEvent, WriteType};
use btleplug::api::{Central as _, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::{Manager};
use futures::StreamExt;
use tokio::sync::watch;
use tokio::time::{self, Instant};
use uuid::Uuid;

use crate::calendar::CalendarInfo;

const TARGET_DEVICE: &str = "DoorSign";
const TARGET_UUID_STR: &str = "6E400002-B5A3-F393-E0A9-E50E24DCCA9E";
const COOLDOWN_SEC: u64 = 30;

pub async fn run(rx: watch::Receiver<Option<CalendarInfo>>) -> ! {
    let uuid = uuid::Uuid::parse_str(TARGET_UUID_STR).expect("Invalid UUID");

    let manager = Manager::new().await.expect("Couldn't create bluetooth manager");
    let adapters = manager.adapters().await.expect("Couldn't get bluetooth adapters");
    let adapter = adapters.first().expect("No bluetooth adapter found");

    let mut filter = ScanFilter::default();
    filter.services.push(uuid);

    adapter.start_scan(filter.clone()).await.expect("Couldn't start bluetooth scan");
    let mut last_update = Instant::now() - Duration::from_secs(COOLDOWN_SEC);
    let mut events = adapter.events().await.expect("Couldn't get bluetooth events");

    println!("Bluetooth listener running");

    while let Some(event) = events.next().await {
        match event {
            CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => {
                let peripheral = adapter.peripheral(&id).await.expect("Couldn't get peripheral");
                let props = match peripheral.properties().await.expect("Couldn't get peripheral properties") {
                    Some(props) => props,
                    None => continue
                };

                let local_name = props.local_name.unwrap_or_default();
                if !local_name.contains(TARGET_DEVICE) {
                    continue;
                }

                if Instant::now() - last_update < Duration::from_secs(COOLDOWN_SEC) {
                    continue;
                }

                println!("Got wakeup call, sending latest data");

                let str_data = match &*rx.borrow() {
                    Some(obj) => serde_json::to_string(obj).unwrap_or_default(),
                    None => continue
                };

                adapter.stop_scan().await.ok();

                if let Err(e) = write_data(&peripheral, str_data.as_bytes(), uuid).await {
                    println!("Error writing calendar data: {}", e);
                } else {
                    println!("Wrote calendar data!");
                    last_update = Instant::now();
                }

                adapter.start_scan(filter.clone()).await.expect("Couldn't start bluetooth scan");
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

    peripheral.write(target_char, data, WriteType::WithResponse).await?;

    peripheral.disconnect().await?;

    Ok(())
}
