use app_state::AppSettings;
use async_trait::async_trait;
use sqlx::PgTransaction;

pub mod cluster_card;
pub mod estimatr_card;
pub mod on_this_day_card;

#[async_trait]
pub trait DailyCardGenerator {
    fn card_type(&self) -> &'static str;
    async fn generate(
        &self,
        tx: &mut PgTransaction<'_>,
        user_id: i32,
        settings: &AppSettings,
    ) -> color_eyre::Result<()>;
}
