//! Neural OCR backend via Candle (requires `ocr-neural` feature).
//!
//! Uses the `candle` deep learning framework to run HuggingFace
//! transformer-based OCR models (e.g. TrOCR) in pure Rust.

use std::path::{Path, PathBuf};

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;

use crate::ocr::{OcrBackend, OcrError, OcrOptions};
use crate::pixmap::Pixmap;
use crate::text::TextLayer;

/// Neural OCR backend using Candle.
///
/// Loads a TrOCR-style encoder-decoder model from safetensors weights.
/// The model must include:
/// - A vision encoder (ViT-based)
/// - A text decoder (GPT2/BART-based)
/// - A tokenizer vocabulary
pub struct CandleBackend {
    device: Device,
    model_dir: PathBuf,
}

impl CandleBackend {
    /// Load a Candle OCR model from a local directory.
    ///
    /// The directory should contain:
    /// - `model.safetensors` — model weights
    /// - `tokenizer.json` — HuggingFace tokenizer
    /// - `config.json` — model configuration
    pub fn load(model_dir: impl AsRef<Path>) -> Result<Self, OcrError> {
        let model_dir = model_dir.as_ref().to_path_buf();
        let device = Device::Cpu;
        Ok(Self { device, model_dir })
    }

    /// Preprocess a pixmap into a normalized RGB tensor.
    fn preprocess(&self, pixmap: &Pixmap) -> Result<Tensor, OcrError> {
        let rgb = pixmap.to_rgb();
        let w = pixmap.width as usize;
        let h = pixmap.height as usize;

        // Normalize to ImageNet mean/std
        let mean = [0.5f32, 0.5, 0.5];
        let std = [0.5f32, 0.5, 0.5];

        let mut data = Vec::with_capacity(3 * h * w);
        for c in 0..3 {
            for i in 0..(h * w) {
                let val = rgb[i * 3 + c] as f32 / 255.0;
                data.push((val - mean[c]) / std[c]);
            }
        }

        // Shape: [1, 3, H, W]
        Tensor::from_vec(data, &[1, 3, h, w], &self.device)
            .map_err(|e| OcrError::RecognitionFailed(format!("tensor creation: {e}")))
    }
}

impl OcrBackend for CandleBackend {
    fn recognize(&self, pixmap: &Pixmap, _options: &OcrOptions) -> Result<TextLayer, OcrError> {
        let _input = self.preprocess(pixmap)?;

        // Load model weights (from_buffered_safetensors is safe; from_mmaped_safetensors is unsafe)
        let weights_path = self.model_dir.join("model.safetensors");
        let weights_data = std::fs::read(&weights_path)
            .map_err(|e| OcrError::InitFailed(format!("weights: {e}")))?;
        let _vb = VarBuilder::from_buffered_safetensors(weights_data, DType::F32, &self.device)
            .map_err(|e| OcrError::InitFailed(format!("weights: {e}")))?;

        // NOTE: Full TrOCR encoder-decoder inference is model-specific.
        // This backend provides the framework; actual model architectures
        // (TrOCR, Donut, Nougat) need dedicated forward pass implementations.
        //
        // For now, return an error indicating the model type is needed.
        // Users should subclass or configure with a specific model architecture.

        Err(OcrError::RecognitionFailed(
            "candle backend requires a model-specific forward pass implementation; \
             see ocr_neural module docs for supported architectures"
                .into(),
        ))
    }
}
