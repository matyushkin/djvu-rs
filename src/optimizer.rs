//! Document-level optimization planning and safe lossless cleanup.
//!
//! The optimizer starts with a deliberately conservative vertical slice:
//! [`OptimizationPreset::LosslessCleanup`] and [`OptimizationPreset::Archival`]
//! remove only IFF `FREE` padding chunks. All image, text, annotation,
//! metadata, bookmark, link, and unknown chunks are preserved byte-for-byte.
//! Archival codec selection and target-size search remain explicit follow-up
//! work; the plan reports that boundary instead of silently recompressing a
//! document.
//!
//! A long run can be observed through [`Optimizer::with_progress`]: the
//! optimizer reports one [`ProgressEvent`] per component in each of the
//! [`OptimizationPhase`]s `plan`, `rewrite` and `verify` (#814). It can be
//! stopped through [`Optimizer::with_cancel`]: the optimizer polls the hook
//! before each component and returns [`OptimizeError::Cancelled`] instead of
//! partial output.

use std::sync::Arc;

use crate::djvu_mut::{DjVuDocumentMut, MutError};
use crate::iff::Chunk;

/// A high-level optimization policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationPreset {
    /// Remove semantically inert IFF padding while preserving decoded pixels.
    LosslessCleanup,
    /// Prefer archival fidelity. This slice applies only the same safe
    /// structural cleanup and never performs an unrequested lossy re-encode.
    Archival,
}

impl OptimizationPreset {
    /// Stable machine-readable spelling used by reports and the CLI.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LosslessCleanup => "lossless-cleanup",
            Self::Archival => "archival",
        }
    }
}

/// Typed optimization constraints.
#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationRequest {
    /// The policy to apply.
    pub preset: OptimizationPreset,
    /// Optional maximum output size in bytes. The first slice reports an
    /// unmet target when safe cleanup alone cannot reach it.
    pub target_size: Option<u64>,
    /// Optional maximum permitted SSIM loss. Reserved for archival re-encode;
    /// the current FREE-cleanup path is pixel-exact by construction and does
    /// not measure SSIM (it warns when this bound is set).
    pub max_ssim_loss: Option<f32>,
}

impl OptimizationRequest {
    /// Construct a request for `preset` with no extra constraints.
    pub const fn new(preset: OptimizationPreset) -> Self {
        Self {
            preset,
            target_size: None,
            max_ssim_loss: None,
        }
    }

    /// Construct the safe lossless-cleanup request.
    pub const fn lossless_cleanup() -> Self {
        Self::new(OptimizationPreset::LosslessCleanup)
    }

    /// Construct an archival-fidelity request.
    pub const fn archival() -> Self {
        Self::new(OptimizationPreset::Archival)
    }

    /// Set a maximum output size in bytes.
    pub const fn with_target_size(mut self, target_size: u64) -> Self {
        self.target_size = Some(target_size);
        self
    }

    /// Set a maximum permitted SSIM loss.
    pub const fn with_max_ssim_loss(mut self, max_ssim_loss: f32) -> Self {
        self.max_ssim_loss = Some(max_ssim_loss);
        self
    }
}

/// The safe structural rewrite selected for a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewriteAction {
    /// Remove a `FREE` padding chunk.
    RemoveFreeChunk,
}

impl RewriteAction {
    const fn as_str(self) -> &'static str {
        match self {
            Self::RemoveFreeChunk => "remove-free-chunk",
        }
    }
}

/// One component changed by an optimization plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewrittenComponent {
    /// Root-relative path into the IFF tree.
    pub path: Vec<usize>,
    /// The four-byte IFF chunk identifier.
    pub chunk_id: [u8; 4],
    /// The safe action selected for the chunk.
    pub action: RewriteAction,
    /// Original payload length in bytes.
    pub input_bytes: usize,
    /// Payload length after the action.
    pub output_bytes: usize,
    /// Human-readable reason for the rewrite.
    pub reason: String,
}

/// A side-effect-free optimization preview.
#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationPlan {
    /// Selected optimization policy.
    pub preset: OptimizationPreset,
    /// Input document size.
    pub input_bytes: usize,
    /// Predicted output document size.
    pub output_bytes: usize,
    /// Number of page components visible in the bundled/single-page input.
    pub page_count: usize,
    /// Whether the selected safe rewrite changes the output bytes.
    pub changed: bool,
    /// Requested output size, if any.
    pub target_size: Option<u64>,
    /// Whether the output satisfies all requested constraints represented by
    /// this slice.
    pub target_met: bool,
    /// Whether the SSIM-loss constraint is satisfied.
    pub quality_floor_met: bool,
    /// Every component this plan rewrites.
    pub rewritten_components: Vec<RewrittenComponent>,
    /// Non-fatal boundaries or unmet constraints.
    pub warnings: Vec<String>,
}

/// Result of applying an optimization plan in memory.
#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationResult {
    /// Optimized bytes. The input is never modified in place.
    pub bytes: Vec<u8>,
    /// Audit report for the applied rewrite.
    pub report: OptimizationReport,
}

/// Audit report emitted after an optimization run.
#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationReport {
    /// Selected optimization policy.
    pub preset: OptimizationPreset,
    /// Input document size.
    pub input_bytes: usize,
    /// Actual output document size.
    pub output_bytes: usize,
    /// Number of page components visible in the input.
    pub page_count: usize,
    /// Whether output bytes differ from input bytes.
    pub changed: bool,
    /// Requested output size, if any.
    pub target_size: Option<u64>,
    /// Whether all represented constraints were satisfied.
    pub target_met: bool,
    /// Whether the SSIM-loss constraint was satisfied.
    pub quality_floor_met: bool,
    /// Every component actually rewritten.
    pub rewritten_components: Vec<RewrittenComponent>,
    /// Non-fatal boundaries or unmet constraints.
    pub warnings: Vec<String>,
}

/// Errors returned while planning or applying an optimization.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OptimizeError {
    /// The input did not parse as an IFF document.
    #[error("optimizer input parse failed: {0}")]
    Parse(#[from] MutError),
    /// A request constraint is invalid.
    #[error("invalid optimization request: {0}")]
    InvalidRequest(&'static str),
    /// The rewritten output did not pass the post-rewrite check.
    ///
    /// The optimizer re-parses what it produced and compares the page count
    /// with the input's. A mismatch means the rewrite is wrong, so the bytes
    /// are withheld rather than returned.
    #[error("optimized output failed verification: {0}")]
    Verification(String),
    /// The run was stopped by the hook installed with
    /// [`Optimizer::with_cancel`]. No output was produced.
    #[error("optimization cancelled")]
    Cancelled,
}

/// The stage of an optimization run a [`ProgressEvent`] belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OptimizationPhase {
    /// Walking the input's components to select rewrites.
    Plan,
    /// Applying the selected rewrites.
    Rewrite,
    /// Re-parsing the output and checking it against the input.
    Verify,
}

impl OptimizationPhase {
    /// Stable machine-readable spelling used by the CLI progress line.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Rewrite => "rewrite",
            Self::Verify => "verify",
        }
    }
}

/// One progress report from an optimization run.
///
/// Delivered to the hook installed with [`Optimizer::with_progress`], on the
/// thread that called [`Optimizer::plan`] or [`Optimizer::optimize`], after
/// the component it names has been handled. Within one phase the index
/// increases by one per event and `bytes_so_far` never decreases.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProgressEvent {
    /// Which stage of the run this event belongs to.
    pub phase: OptimizationPhase,
    /// Zero-based position of the component within its phase.
    pub component_index: usize,
    /// How many components this phase handles in total.
    pub component_count: usize,
    /// Four-byte IFF identifier of the component: the secondary ID of a
    /// `FORM` (`DJVU`, `DJVI`, `THUM`), or the chunk ID of a leaf (`DIRM`,
    /// `NAVM`, `FREE`).
    pub component_id: [u8; 4],
    /// Bytes accounted for so far in this phase, this component included.
    ///
    /// In `plan` and `verify` that is the encoded size of the components
    /// walked; in `rewrite` it is the input payload of the components
    /// rewritten.
    pub bytes_so_far: usize,
}

/// A progress hook shared by an [`Optimizer`] and its clones.
pub type ProgressHook = Arc<dyn Fn(&ProgressEvent) + Send + Sync>;

/// A cancellation hook shared by an [`Optimizer`] and its clones. It returns
/// `true` once the run should stop.
pub type CancelHook = Arc<dyn Fn() -> bool + Send + Sync>;

/// High-level optimizer configured with one typed request.
///
/// The optimizer stays `UnwindSafe` and `RefUnwindSafe` with hooks
/// installed: it holds no state a panic can leave half-updated, and a
/// panicking hook unwinds through a run that borrows the optimizer only
/// immutably. The explicit impls below record that reasoning; the `dyn Fn`
/// behind a hook would otherwise drop both auto traits.
#[derive(Clone)]
pub struct Optimizer {
    request: OptimizationRequest,
    on_progress: Option<ProgressHook>,
    cancelled: Option<CancelHook>,
}

impl std::panic::UnwindSafe for Optimizer {}
impl std::panic::RefUnwindSafe for Optimizer {}

impl core::fmt::Debug for Optimizer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Optimizer")
            .field("request", &self.request)
            .field("on_progress", &self.on_progress.is_some())
            .field("cancelled", &self.cancelled.is_some())
            .finish()
    }
}

impl Optimizer {
    /// Create an optimizer from a typed request.
    pub const fn new(request: OptimizationRequest) -> Self {
        Self {
            request,
            on_progress: None,
            cancelled: None,
        }
    }

    /// Install a progress hook.
    ///
    /// `hook` receives one [`ProgressEvent`] per component per phase, on the
    /// calling thread. [`Optimizer::plan`] reports the `plan` phase;
    /// [`Optimizer::optimize`] reports `plan`, then `rewrite`, then `verify`.
    /// A phase with nothing to do (no rewrites selected) reports no events.
    /// The hook lives on the optimizer, not on [`OptimizationRequest`], so the
    /// request stays a plain comparable value.
    pub fn with_progress<F>(mut self, hook: F) -> Self
    where
        F: Fn(&ProgressEvent) + Send + Sync + 'static,
    {
        self.on_progress = Some(Arc::new(hook));
        self
    }

    /// Install a cancellation hook.
    ///
    /// `hook` is polled on the calling thread before the input is parsed and
    /// before each component of each phase, the same cooperative contract as
    /// [`crate::export_control::ExportObserver::cancelled`]. Once it returns
    /// `true`, [`Optimizer::plan`] and [`Optimizer::optimize`] return
    /// [`OptimizeError::Cancelled`]. Work already begun on a component
    /// completes first; no partial output is ever returned.
    pub fn with_cancel<F>(mut self, hook: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.cancelled = Some(Arc::new(hook));
        self
    }

    /// Stop the run here if the cancellation hook asks for it.
    fn check_cancelled(&self) -> Result<(), OptimizeError> {
        match &self.cancelled {
            Some(hook) if hook() => Err(OptimizeError::Cancelled),
            _ => Ok(()),
        }
    }

    fn report_progress(&self, event: ProgressEvent) {
        if let Some(hook) = &self.on_progress {
            hook(&event);
        }
    }

    /// Walk every component of `document` under `phase`, in order: poll for
    /// cancellation before each one, report it after.
    fn walk_components(
        &self,
        phase: OptimizationPhase,
        document: &DjVuDocumentMut,
    ) -> Result<(), OptimizeError> {
        if self.on_progress.is_none() && self.cancelled.is_none() {
            return Ok(());
        }
        let components = components(document);
        let mut bytes_so_far = 0usize;
        for (index, (id, bytes)) in components.iter().enumerate() {
            self.check_cancelled()?;
            bytes_so_far += bytes;
            self.report_progress(ProgressEvent {
                phase,
                component_index: index,
                component_count: components.len(),
                component_id: *id,
                bytes_so_far,
            });
        }
        Ok(())
    }

    /// Inspect the input and produce a side-effect-free rewrite plan.
    pub fn plan(&self, input: &[u8]) -> Result<OptimizationPlan, OptimizeError> {
        self.validate_request()?;
        self.check_cancelled()?;
        let document = DjVuDocumentMut::from_bytes(input)?;
        self.walk_components(OptimizationPhase::Plan, &document)?;
        let mut candidates = Vec::new();
        let mut path = Vec::new();
        collect_free_chunks(document.root_chunk(), &mut path, &mut candidates);

        let reason = match self.request.preset {
            OptimizationPreset::LosslessCleanup => {
                "FREE is semantically inert IFF padding and can be removed without decoding pixels"
            }
            OptimizationPreset::Archival => {
                "archival policy currently selects only safe FREE-padding cleanup"
            }
        };
        let rewritten_components = candidates
            .into_iter()
            .map(|candidate| RewrittenComponent {
                path: candidate.path,
                chunk_id: *b"FREE",
                action: RewriteAction::RemoveFreeChunk,
                input_bytes: candidate.payload_bytes,
                output_bytes: 0,
                reason: reason.to_string(),
            })
            .collect::<Vec<_>>();

        // A dry application, to size the output; it is polled for
        // cancellation like every other per-component step.
        let output = apply_rewrites(
            &document,
            &rewritten_components,
            |_, _| self.check_cancelled(),
            |_, _| {},
        )?;
        let output_bytes = output.len();
        // FREE removal is pixel-exact by construction. SSIM measurement applies
        // only once archival re-encode exists; keep the floor "met" here and
        // warn when the caller supplied a threshold expecting a future gate.
        let quality_floor_met = true;
        let target_size_met = self
            .request
            .target_size
            .is_none_or(|target| output_bytes as u64 <= target);
        let target_met = target_size_met && quality_floor_met;
        let mut warnings = Vec::new();
        if self.request.max_ssim_loss.is_some() {
            warnings.push(
                "max_ssim_loss is reserved for archival re-encode; lossless FREE cleanup is pixel-exact by construction and does not measure SSIM".to_string(),
            );
        }
        if matches!(self.request.preset, OptimizationPreset::Archival) {
            warnings.push(
                "archival codec re-encode, quality search, and cancellation are not yet selected; output remains pixel-exact".to_string(),
            );
        }
        if !target_size_met {
            let target = self.request.target_size.unwrap_or_default();
            warnings.push(format!(
                "target size {target} bytes cannot be met by safe structural cleanup; output is {output_bytes} bytes"
            ));
        }

        Ok(OptimizationPlan {
            preset: self.request.preset,
            input_bytes: input.len(),
            output_bytes,
            page_count: page_count(&document),
            changed: output != input,
            target_size: self.request.target_size,
            target_met,
            quality_floor_met,
            rewritten_components,
            warnings,
        })
    }

    /// Apply the selected plan in memory and return bytes plus an audit report.
    pub fn optimize(&self, input: &[u8]) -> Result<OptimizationResult, OptimizeError> {
        let plan = self.plan(input)?;
        let document = DjVuDocumentMut::from_bytes(input)?;
        let mut bytes_so_far = 0usize;
        let bytes = apply_rewrites(
            &document,
            &plan.rewritten_components,
            |_, _| self.check_cancelled(),
            |index, component| {
                bytes_so_far += component.input_bytes;
                self.report_progress(ProgressEvent {
                    phase: OptimizationPhase::Rewrite,
                    component_index: index,
                    component_count: plan.rewritten_components.len(),
                    component_id: component.chunk_id,
                    bytes_so_far,
                });
            },
        )?;
        self.check_cancelled()?;
        let output = DjVuDocumentMut::from_bytes(&bytes)
            .map_err(|e| OptimizeError::Verification(format!("output does not parse: {e}")))?;
        self.walk_components(OptimizationPhase::Verify, &output)?;
        let output_pages = page_count(&output);
        if output_pages != plan.page_count {
            return Err(OptimizeError::Verification(format!(
                "input has {} pages, output has {output_pages}",
                plan.page_count
            )));
        }
        let report = OptimizationReport {
            preset: plan.preset,
            input_bytes: plan.input_bytes,
            output_bytes: bytes.len(),
            page_count: plan.page_count,
            changed: bytes != input,
            target_size: plan.target_size,
            target_met: plan.target_met,
            quality_floor_met: plan.quality_floor_met,
            rewritten_components: plan.rewritten_components,
            warnings: plan.warnings,
        };
        Ok(OptimizationResult { bytes, report })
    }

    fn validate_request(&self) -> Result<(), OptimizeError> {
        if let Some(loss) = self.request.max_ssim_loss
            && (!loss.is_finite() || loss < 0.0)
        {
            return Err(OptimizeError::InvalidRequest(
                "max_ssim_loss must be a finite non-negative number",
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct FreeCandidate {
    path: Vec<usize>,
    payload_bytes: usize,
}

fn collect_free_chunks(chunk: &Chunk, path: &mut Vec<usize>, candidates: &mut Vec<FreeCandidate>) {
    match chunk {
        Chunk::Form { children, .. } => {
            for (index, child) in children.iter().enumerate() {
                path.push(index);
                collect_free_chunks(child, path, candidates);
                path.pop();
            }
        }
        Chunk::Leaf { id, data } => {
            if id == b"FREE" {
                candidates.push(FreeCandidate {
                    path: path.clone(),
                    payload_bytes: data.len(),
                });
            }
        }
    }
}

/// Apply `rewrites` to a copy of `document`, one component at a time in plan
/// order. `before` runs ahead of each rewrite and can stop the run; `after`
/// runs once the rewrite is applied.
///
/// Plan paths name positions in the *input*. Removing a leaf shifts the later
/// siblings of the same parent down by one, so each path is adjusted by the
/// removals already made ahead of it. Rewrites only remove leaves, so no
/// removed path is a prefix of another.
fn apply_rewrites(
    document: &DjVuDocumentMut,
    rewrites: &[RewrittenComponent],
    mut before: impl FnMut(usize, &RewrittenComponent) -> Result<(), OptimizeError>,
    mut after: impl FnMut(usize, &RewrittenComponent),
) -> Result<Vec<u8>, OptimizeError> {
    let mut edited = document.clone();
    let mut removed: Vec<&[usize]> = Vec::with_capacity(rewrites.len());
    for (index, component) in rewrites.iter().enumerate() {
        before(index, component)?;
        let path = &component.path;
        let Some((&last, parent)) = path.split_last() else {
            return Err(OptimizeError::InvalidRequest("a rewrite path is empty"));
        };
        let shift = removed
            .iter()
            .filter(|done| done.len() == path.len() && done.starts_with(parent))
            .filter(|done| done[done.len() - 1] < last)
            .count();
        let mut current = path.clone();
        current[path.len() - 1] = last - shift;
        edited.remove_leaf(&current)?;
        removed.push(path);
        after(index, component);
    }
    Ok(edited.try_into_bytes()?)
}

fn page_count(document: &DjVuDocumentMut) -> usize {
    match document.root_form_type() {
        Some(form_type) if *form_type == *b"DJVU" => 1,
        Some(form_type) if *form_type == *b"DJVM" => (0..document.root_child_count())
            .filter_map(|index| document.chunk_at_path(&[index]).ok())
            .filter(|chunk| {
                matches!(chunk, Chunk::Form { secondary_id, .. } if secondary_id == b"DJVU")
            })
            .count(),
        _ => 0,
    }
}

/// The components progress is reported over: the root's children of a bundled
/// `DJVM`, or the root itself for a single-page `DJVU` (and any other root).
/// Each entry is the component's IFF identifier and its encoded size, header
/// included.
fn components(document: &DjVuDocumentMut) -> Vec<([u8; 4], usize)> {
    const HEADER: usize = 8;
    let id_and_size = |chunk: &Chunk| -> ([u8; 4], usize) {
        let id = match chunk {
            Chunk::Form { secondary_id, .. } => *secondary_id,
            Chunk::Leaf { id, .. } => *id,
        };
        (id, HEADER + chunk.payload_length() as usize)
    };
    match document.root_form_type() {
        Some(form_type) if *form_type == *b"DJVM" => (0..document.root_child_count())
            .filter_map(|index| document.chunk_at_path(&[index]).ok())
            .map(id_and_size)
            .collect(),
        _ => vec![id_and_size(document.root_chunk())],
    }
}

impl OptimizationPlan {
    /// Serialize the plan as stable, dependency-free JSON.
    pub fn to_json(&self) -> String {
        json_for(&JsonSummary {
            preset: self.preset,
            input_bytes: self.input_bytes,
            output_bytes: self.output_bytes,
            page_count: self.page_count,
            changed: self.changed,
            target_size: self.target_size,
            target_met: self.target_met,
            quality_floor_met: self.quality_floor_met,
            rewritten_components: &self.rewritten_components,
            warnings: &self.warnings,
        })
    }
}

impl OptimizationReport {
    /// Serialize the report as stable, dependency-free JSON.
    pub fn to_json(&self) -> String {
        json_for(&JsonSummary {
            preset: self.preset,
            input_bytes: self.input_bytes,
            output_bytes: self.output_bytes,
            page_count: self.page_count,
            changed: self.changed,
            target_size: self.target_size,
            target_met: self.target_met,
            quality_floor_met: self.quality_floor_met,
            rewritten_components: &self.rewritten_components,
            warnings: &self.warnings,
        })
    }
}

struct JsonSummary<'a> {
    preset: OptimizationPreset,
    input_bytes: usize,
    output_bytes: usize,
    page_count: usize,
    changed: bool,
    target_size: Option<u64>,
    target_met: bool,
    quality_floor_met: bool,
    rewritten_components: &'a [RewrittenComponent],
    warnings: &'a [String],
}

fn json_for(summary: &JsonSummary<'_>) -> String {
    let target = summary
        .target_size
        .map_or_else(|| "null".to_string(), |value| value.to_string());
    let components = summary
        .rewritten_components
        .iter()
        .map(|component| {
            format!(
                "{{\"path\":{},\"chunk_id\":\"{}\",\"action\":\"{}\",\"input_bytes\":{},\"output_bytes\":{},\"reason\":\"{}\"}}",
                json_path(&component.path),
                json_escape(&String::from_utf8_lossy(&component.chunk_id)),
                component.action.as_str(),
                component.input_bytes,
                component.output_bytes,
                json_escape(&component.reason),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let warning_json = summary
        .warnings
        .iter()
        .map(|warning| format!("\"{}\"", json_escape(warning)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"preset\":\"{}\",\"input_bytes\":{},\"output_bytes\":{},\"page_count\":{},\"changed\":{},\"target_size\":{},\"target_met\":{},\"quality_floor_met\":{},\"rewritten_components\":[{}],\"warnings\":[{}]}}",
        summary.preset.as_str(),
        summary.input_bytes,
        summary.output_bytes,
        summary.page_count,
        summary.changed,
        target,
        summary.target_met,
        summary.quality_floor_met,
        components,
        warning_json,
    )
}

fn json_path(path: &[usize]) -> String {
    format!(
        "[{}]",
        path.iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The semver gate compares auto traits against the published crate. An
    /// installed hook must not cost `Optimizer` its unwind safety (#814).
    #[test]
    fn optimizer_stays_unwind_safe_with_hooks() {
        fn assert_unwind_safe<T: std::panic::UnwindSafe + std::panic::RefUnwindSafe>(_: &T) {}
        let optimizer = Optimizer::new(OptimizationRequest::lossless_cleanup())
            .with_progress(|_| {})
            .with_cancel(|| false);
        assert_unwind_safe(&optimizer);
        let debug = format!("{optimizer:?}");
        assert!(debug.contains("on_progress: true"));
        assert!(debug.contains("cancelled: true"));
    }
}
