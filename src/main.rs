use std::time::Duration;

use btleplug::api::{Central as _, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::Manager;
use tokio::time;

const TARGET_DEVICE: &str = "DoorSign";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let uuid = uuid::Builder::from_u128(0);
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters.iter().nth(0).expect("No bluetooth adapter found");

    let info = adapter.adapter_info().await?;

    println!("Found bluetooth device: {}", info);

    if let Err(e) = adapter.start_scan(ScanFilter::default()).await {
        eprintln!("Couldn't start bt scan: {}", e);
    }

    time::sleep(Duration::from_secs(2)).await;

    let peripherals = adapter.peripherals().await?;

    for p in &peripherals {
        let properties = p.properties().await?;
        let is_target = properties.iter().any(|x| {
            x.local_name
                .as_ref()
                .is_some_and(|k| k.contains(TARGET_DEVICE))
        });

        if is_target {}
    }

    adapter.stop_scan().await?;

    Ok(())
}
