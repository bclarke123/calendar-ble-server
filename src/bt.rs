use std::time::Duration;

use anyhow::Error;
use btleplug::api::WriteType;
use btleplug::api::{Central as _, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::Manager;
use tokio::sync::watch::Receiver;
use tokio::time;

use crate::calendar::CalendarInfo;

const TARGET_DEVICE: &str = "DoorSign";

pub async fn watch_for_device(mut rx: Receiver<Option<CalendarInfo<'_>>>) -> ! {
    loop {
        rx.changed().await.ok();

        let obj = {
            let obj = *rx.borrow();
            if obj.is_none() {
                println!("New calendar info is empty");
                continue;
            }

            obj.unwrap()
        };

        println!("New calendar info, sending to device");

        let ser = serde_json::to_string(&obj);
        if let Err(e) = ser {
            println!("Couldn't serialize calendar info: {}", e); // ???
            continue;
        }

        let str = ser.unwrap();

        for _ in 0..10 {
            match write_data(str.as_bytes()).await {
                Ok(_) => {
                    println!("Updated!");
                    break;
                }
                Err(e) => {
                    println!("Error sending data: {}", e);
                }
            }
        }
    }
}

pub async fn write_data(data: &[u8]) -> anyhow::Result<()> {
    let uuid = uuid::Builder::from_u128(0).into_uuid();
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters.first().expect("No bluetooth adapter found");
    let info = adapter.adapter_info().await?;

    println!("Found bluetooth device: {}", info);

    if let Err(e) = adapter.start_scan(ScanFilter::default()).await {
        eprintln!("Couldn't start bt scan: {}", e);
    }

    time::sleep(Duration::from_secs(2)).await;

    let peripherals = adapter.peripherals().await?;

    let mut sent = false;

    for p in &peripherals {
        let properties = p.properties().await?;
        let is_target = properties.iter().any(|x| {
            x.local_name
                .as_ref()
                .is_some_and(|k| k.contains(TARGET_DEVICE))
        });

        if !is_target {
            continue;
        }

        println!("Found target device, connecting");

        adapter.stop_scan().await?;

        if let Err(e) = p.connect().await {
            println!("Couldn't connect to target device: {}", e);
            continue;
        }

        p.discover_services().await?;

        let chars = p.characteristics();
        let target_char = chars.iter().find(|x| x.uuid == uuid);

        if target_char.is_none() {
            println!("Couln't find characteristic on target device: {}", uuid);
            continue;
        }

        let target_char = target_char.unwrap();

        println!("Writing status...");

        if let Err(e) = p.write(target_char, data, WriteType::WithResponse).await {
            println!("Error writing to characteristic: {}", e);
        } else {
            println!("Write successful!");
            sent = true;
        }

        p.disconnect().await?;
    }

    adapter.stop_scan().await?;

    if sent {
        Ok(())
    } else {
        Err(Error::msg("Couldn't find device"))
    }
}
