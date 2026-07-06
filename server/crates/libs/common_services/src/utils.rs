/// Generate a URL-safe random ID of a given length.
#[must_use]
pub fn nice_id(length: usize) -> String {
    const URL_SAFE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_";
    (0..length)
        .map(|_| {
            let idx = rand::random_range(0..URL_SAFE.len());
            URL_SAFE[idx] as char
        })
        .collect()
}

/// Logs a warning message with an 'ALERT:' prefix.
#[macro_export]
macro_rules! alert {
    ($($arg:tt)*) => {
        warn!("ALERT: {}", format_args!($($arg)*));
    };
}
