pub fn today_yyyy_mm_dd() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}