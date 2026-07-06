use crate::color_data::get_color_data;
use crate::get_llm_classification;
use crate::quality_judge::get_quality_judgement;
use crate::quality_measure::get_quality_measurement;
use crate::utils::convert_media_file;
use app_state::AnalyzerSettings;
use color_eyre::eyre::eyre;
use common_types::ml_analysis::{MLChatAnalysis, MLFastAnalysis};
use face_id::analyzer::FaceAnalyzer;
use language_model::{ChatSession, LlamaClient};
use object_detector::{DetectorType, ModelScale, ObjectDetector};
use open_clip_inference::VisionEmbedder;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tempfile::Builder;

pub struct VisualAnalyzer {
    pub llm_client: LlamaClient,
    pub embedder: Arc<VisionEmbedder>,
    pub face_analyzer: Arc<FaceAnalyzer>,
    pub object_detector: Arc<ObjectDetector>,
}

impl VisualAnalyzer {
    /// Creates a new instance of the `VisualAnalyzer`.
    pub async fn new(embedder_model_id: &str) -> color_eyre::Result<Self> {
        let llm = LlamaClient::with_base_url("http://localhost:8080").build();
        let embedder = VisionEmbedder::from_hf(embedder_model_id).build().await?;
        let face_analyzer = FaceAnalyzer::from_hf().build().await?;
        let object_detector = ObjectDetector::from_hf(DetectorType::PromptFree)
            .scale(ModelScale::Large)
            .include_mask(false)
            .build()
            .await?;
        Ok(Self {
            llm_client: llm,
            embedder: Arc::new(embedder),
            face_analyzer: Arc::new(face_analyzer),
            object_detector: Arc::new(object_detector),
        })
    }

    /// Get stateful llm session from llm client
    #[must_use]
    pub fn get_llm_session(&self) -> ChatSession {
        ChatSession::new(self.llm_client.clone())
    }

    async fn get_analysis_file(
        file: &Path,
        analyze_image_size: u64,
    ) -> color_eyre::Result<PathBuf> {
        let Some(extension) = file.extension().map(|e| e.to_string_lossy().to_string()) else {
            return Err(eyre!("Can't get extension from file"));
        };

        let mut analysis_file = file.to_path_buf();
        if !["jpg", "jpeg"].contains(&&*extension.to_lowercase()) {
            let temp_file = Builder::new()
                .suffix(".jpg")
                .disable_cleanup(true)
                .tempfile()?;
            analysis_file = temp_file.path().to_path_buf();
            convert_media_file(file, &analysis_file, analyze_image_size).await?;
        }
        Ok(analysis_file)
    }

    /// Performs a visual analysis of the given image file, extracting various data points like color, embedding, faces, and objects.
    ///
    /// # Errors
    ///
    /// Returns an error if the file extension cannot be determined, if file conversion to JPEG fails, or if any of the underlying analysis steps encounter an error.
    pub async fn fast_image_analysis(
        &self,
        config: &AnalyzerSettings,
        file: &Path,
        percentage: i32,
    ) -> color_eyre::Result<MLFastAnalysis> {
        let start = Instant::now();
        let analysis_file = Self::get_analysis_file(file, config.analyze_image_size).await?;
        println!("get_analysis_file {:?}", start.elapsed());
        let img = image::open(&analysis_file)?;

        let now = Instant::now();
        let color_data = get_color_data(&img)?;
        println!("color_data {:?}", now.elapsed());

        let now = Instant::now();
        let embedding = self.embedder.embed_image(&img).map(|e| e.to_vec())?;
        println!("embedding {:?}", now.elapsed());

        let now = Instant::now();
        let faces = self.face_analyzer.analyze(&img)?;
        println!("faces {:?}", now.elapsed());

        let now = Instant::now();
        let objects = self
            .object_detector
            .predict(&img)
            .confidence_threshold(0.4)
            .call()?;
        println!("objects {:?}", now.elapsed());

        let now = Instant::now();
        let measured_quality = get_quality_measurement(&analysis_file)?;
        println!("get_quality_measurement {:?}", now.elapsed());

        let _ = tokio::fs::remove_file(&analysis_file).await;

        println!("-- Fast ml analysis {:?}", start.elapsed());

        Ok(MLFastAnalysis {
            percentage,
            color_data,
            embedding,
            faces,
            objects,
            measured_quality,
        })
    }

    /// Performs a visual analysis of the given image file, extracting various data points like color, quality, and content.
    ///
    /// # Errors
    ///
    /// Returns an error if the file extension cannot be determined, if file conversion to JPEG fails, or if any of the underlying analysis steps encounter an error.
    pub async fn llm_analysis(
        &self,
        config: &AnalyzerSettings,
        file: &Path,
        percentage: i32,
    ) -> color_eyre::Result<MLChatAnalysis> {
        let start = Instant::now();
        let now = Instant::now();
        let analysis_file = Self::get_analysis_file(file, config.analyze_image_size).await?;
        println!("Convert to jpg {:?}", now.elapsed());

        let now = Instant::now();
        let llm_classification = get_llm_classification(&self.llm_client, &analysis_file).await?;
        println!("get_caption_data {:?}", now.elapsed());

        let now = Instant::now();
        let quality_judge = get_quality_judgement(&self.llm_client, &analysis_file).await?;
        println!("get_quality_judgement {:?}", now.elapsed());

        tokio::fs::remove_file(&analysis_file).await?;

        println!("total ml analysis {:?}", start.elapsed());

        Ok(MLChatAnalysis {
            percentage,
            quality_judge,
            llm_classification,
        })
    }
}
