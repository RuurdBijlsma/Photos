#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::large_futures
)]

#[cfg(test)]
pub mod runner;
#[cfg(test)]
pub mod test_helpers;
#[cfg(test)]
pub mod tests;

#[cfg(test)]
mod test_runner {
    use crate::runner::context::test_context::TestContext;
    use crate::runner::orchestration_utils::setup_tracing_and_panic_handling;
    use crate::tests::test_album::{
        test_album_lifecycle, test_album_media_management, test_album_rank_override,
        test_album_sharing, test_manual_album_concurrent_add, test_update_album,
    };
    use crate::tests::test_auth::{
        test_login, test_logout, test_refresh, test_register, test_second_register_attempt,
    };
    use crate::tests::test_jobs::{
        test_bulk_enqueue_full_ingest_concurrent_overlap, test_cancel_all_ingest_jobs,
        test_cancelled_job_is_not_resurrected, test_clean_thumbnails_removes_orphans,
        test_permanently_failed_ingest_blocks_repair, test_scan_enqueues_sync_ingest,
        test_sync_ingest_is_user_scoped, test_sync_ingest_repair_is_user_scoped,
        test_sync_ingest_repairs_incomplete_items,
    };
    use crate::tests::test_onboarding::{test_onboarding, test_start_processing};
    use crate::tests::test_photos::{
        test_get_color_theme, test_get_full_item, test_get_full_item_albums, test_get_random_photo,
        test_photo_download,
    };
    use crate::tests::test_root::test_health_endpoint;
    use crate::tests::test_search::test_search_filters;
    use crate::tests::test_takeout::test_takeout_import_flow;
    use crate::tests::test_timeline::{
        test_get_photos_by_month, test_get_timeline_ids, test_get_timeline_ratios,
    };
    use crate::{execute_suite, run_test};
    use color_eyre::Result;
    use colored::*;
    use std::time::Instant;

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn integration_suite() -> Result<()> {
        setup_tracing_and_panic_handling();
        let context = TestContext::new().await?;

        execute_suite!(
            &context,
            [
                // -- Root --
                test_health_endpoint,
                // -- Auth --
                test_register,
                test_second_register_attempt,
                test_login,
                test_refresh,
                test_logout,
                // -- Onboarding --
                test_onboarding,
                test_start_processing,
                // -- Timeline --
                test_get_timeline_ids,
                test_get_timeline_ratios,
                test_get_photos_by_month,
                // -- Albums --
                test_album_lifecycle,
                test_update_album,
                test_album_media_management,
                test_album_sharing,
                test_album_rank_override,
                test_manual_album_concurrent_add,
                // -- Photos --
                test_photo_download,
                test_get_full_item,
                test_get_full_item_albums,
                test_get_color_theme,
                test_get_random_photo,
                // -- Search --
                test_search_filters,
                // -- Takeout Import --
                test_takeout_import_flow,
                // -- Jobs --
                test_cancel_all_ingest_jobs,
                test_cancelled_job_is_not_resurrected,
                test_bulk_enqueue_full_ingest_concurrent_overlap,
                test_scan_enqueues_sync_ingest,
                test_sync_ingest_is_user_scoped,
                test_sync_ingest_repair_is_user_scoped,
                test_clean_thumbnails_removes_orphans,
                test_sync_ingest_repairs_incomplete_items,
                test_permanently_failed_ingest_blocks_repair,
            ]
        );

        Ok(())
    }
}
