#![deny(clippy::unwrap_used)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

mod classify;
mod color_data;
mod quality_judge;
mod quality_measure;
mod utils;
mod visual_analyzer;

pub use classify::get_llm_classification;
pub use color_data::get_color_data;
pub use quality_judge::get_quality_judgement;
pub use quality_measure::get_quality_measurement;
pub use quality_measure::get_quality_measurement_from_image;
pub use quality_measure::get_quality_measurement_from_luma;
pub use utils::*;
pub use visual_analyzer::VisualAnalyzer;
