use cosmwasm_std::Timestamp;

/// Normalize any timestamp to midnight UTC of that day.
pub fn normalize_to_date(timestamp: Timestamp) -> Timestamp {
    // 86400 seconds in a day
    let seconds = timestamp.seconds();
    let days = seconds / 86400;
    Timestamp::from_seconds(days * 86400)
}
