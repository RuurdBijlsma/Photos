// simple exponential backoff: 2^attempt * 10 seconds
#[must_use]
pub fn backoff_seconds(attempts: i32) -> i64 {
    #[allow(clippy::cast_sign_loss)]
    let secs = 10 * (2_i64.pow(attempts as u32));
    secs.min(3600) // cap at 1h
}

#[macro_export]
macro_rules! insert_query {
    (
        $tx:expr,
        $table:literal,
        { $($col:ident: $val:expr),+ $(,)? }
    ) => {
        async {
            let columns = stringify!($($col),+);
            let placeholders = (1..=$crate::count_tts!($($col)+))
                .map(|i| format!("${}", i))
                .collect::<Vec<_>>()
                .join(", ");

            let sql = format!("INSERT INTO {} ({}) VALUES ({})", $table, columns, placeholders);

            sqlx::query(&sql)
            $(
                .bind($val)
            )+
            .execute(&mut **$tx)
            .await
        }
    };
}

/// Helper macro to count the number of tokens.
#[macro_export]
macro_rules! count_tts {
    ($($tts:tt)*) => {<[()]>::len(&[$($crate::replace_expr!($tts ())),*])};
}

/// Helper macro for `count_tts`.
#[macro_export]
macro_rules! replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}
