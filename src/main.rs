mod bt;
mod calendar;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let status = calendar::CalendarInfo::fetch_current_status().await?;
    let ser = serde_json::to_string(&status)?;

    println!("{}", ser);

    bt::write_data("Whatever".as_bytes()).await?;
    Ok(())
}
