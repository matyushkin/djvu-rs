//! Neural OCR backend (requires `ocr-neural` feature).
//!
//! > ⚠️ **Not yet implemented.** The `recognize` method always returns
//! > `Err(OcrError::RecognitionFailed)`. Enabling `ocr-neural` adds **no**
//! > heavy dependencies — the feature is a lightweight stub.
//! >
//! > To experiment with actual Candle/TrOCR inference, enable the
//! > **`ocr-neural-candle`** feature, which additionally pulls in
//! > `candle-core`, `candle-nn`, and `tokenizers` (~400 transitive crates).
//! > See <https://github.com/matyushkin/djvu-rs/issues/162>.

use std::path::{Path, PathBuf};

use crate::ocr::{OcrBackend, OcrError, OcrOptions};
use crate::pixmap::Pixmap;
use crate::text::TextLayer;

/// Neural OCR backend using Candle.
///
/// With only the `ocr-neural` feature every call to
/// [`recognize`][OcrBackend::recognize] returns `Err`.  Enable
/// `ocr-neural-candle` to additionally compile the Candle tensor pipeline;
/// it still returns `Err` until a model-specific forward pass is wired in.
pub struct CandleBackend {
    // Only accessed by the ocr-neural-candle feature; suppress dead_code otherwise.
    #[cfg_attr(not(feature = "ocr-neural-candle"), allow(dead_code))]
    model_dir: PathBuf,
}

impl CandleBackend {
    /// Load a (future) Candle OCR model from a local directory.
    ///
    /// Expected contents when a real model is wired up:
    /// - `model.safetensors` — model weights
    /// - `tokenizer.json` — HuggingFace tokenizer
    /// - `config.json` — model configuration
    pub fn load(model_dir: impl AsRef<Path>) -> Result<Self, OcrError> {
        Ok(Self {
            model_dir: model_dir.as_ref().to_path_buf(),
        })
    }
}

impl OcrBackend for CandleBackend {
    fn recognize(&self, pixmap: &Pixmap, _options: &OcrOptions) -> Result<TextLayer, OcrError> {
        // With ocr-neural-candle, attempt the tensor pipeline (still returns Err
        // until a full model forward pass is implemented).
        #[cfg(feature = "ocr-neural-candle")]
        {
            let input = self.preprocess(pixmap)?;
            return self.run_inference(&input);
        }

        // Without candle, return a clear error immediately.
        #[cfg(not(feature = "ocr-neural-candle"))]
        {
            let _ = pixmap;
            return Err(OcrError::RecognitionFailed(
                "candle backend requires the `ocr-neural-candle` feature; \
                 see ocr_neural module docs"
                    .into(),
            ));
        }
    }
}

// ---- Candle tensor pipeline (compiled only with ocr-neural-candle) ----------

#[cfg(feature = "ocr-neural-candle")]
impl CandleBackend {
    /// Preprocess a pixmap into a normalized RGB tensor `[1, 3, H, W]`.
    fn preprocess(&self, pixmap: &Pixmap) -> Result<candle_core::Tensor, OcrError> {
        use candle_core::{Device, Tensor};

        let device = Device::Cpu;
        let rgb = pixmap.to_rgb();
        let w = pixmap.width as usize;
        let h = pixmap.height as usize;

        let mean = [0.5f32, 0.5, 0.5];
        let std = [0.5f32, 0.5, 0.5];

        let mut data = Vec::with_capacity(3 * h * w);
        for c in 0..3 {
            for i in 0..(h * w) {
                let val = rgb[i * 3 + c] as f32 / 255.0;
                data.push((val - mean[c]) / std[c]);
            }
        }

        Tensor::from_vec(data, &[1, 3, h, w], &device)
            .map_err(|e| OcrError::RecognitionFailed(format!("tensor creation: {e}")))
    }

    /// Stub forward pass — loads weights and returns Err until a real
    /// architecture (TrOCR, Donut, Nougat …) is implemented.
    fn run_inference(&self, _input: &candle_core::Tensor) -> Result<TextLayer, OcrError> {
        use candle_core::{DType, Device};
        use candle_nn::VarBuilder;

        let weights_path = self.model_dir.join("model.safetensors");
        let weights_data = std::fs::read(&weights_path)
            .map_err(|e| OcrError::InitFailed(format!("weights: {e}")))?;
        let _vb = VarBuilder::from_buffered_safetensors(weights_data, DType::F32, &Device::Cpu)
            .map_err(|e| OcrError::InitFailed(format!("weights: {e}")))?;

        Err(OcrError::RecognitionFailed(
            "candle backend requires a model-specific forward pass; \
             see ocr_neural module docs for supported architectures"
                .into(),
        ))
    }
}
