use futures::future::join;
use tokio::sync::watch;

use crate::calendar::CalendarInfo;

mod bt;
mod calendar;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, rx) = watch::channel::<Option<CalendarInfo>>(None);

    let bt_handle = tokio::spawn(bt::watch_for_device(rx));
    let cal_handle = tokio::spawn(calendar::sync_task(tx));

    let _ = join(bt_handle, cal_handle).await;

    Ok(())
}
