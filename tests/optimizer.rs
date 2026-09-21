//! TDD coverage for the document optimizer's first lossless-cleanup slice.

use std::fs;

use assert_cmd::Command;
use djvu_rs::Bitmap;
use djvu_rs::djvu_encode::PageEncoder;
use djvu_rs::iff::{self, Chunk};
use djvu_rs::optimizer::{
    OptimizationPhase, OptimizationPreset, OptimizationRequest, OptimizeError, Optimizer,
    ProgressEvent,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

fn page_with_free_and_unknown_chunk() -> Vec<u8> {
    let bitmap = Bitmap::new(8, 8);
    let encoded = PageEncoder::from_bitmap(&bitmap).encode().unwrap();
    let mut file = iff::parse(&encoded).unwrap();
    match &mut file.root {
        Chunk::Form { children, .. } => {
            children.insert(
                1,
                Chunk::Leaf {
                    id: *b"FREE",
                    data: vec![0; 17],
                },
            );
            children.push(Chunk::Leaf {
                id: *b"Xtra",
                data: b"preserve me".to_vec(),
            });
        }
        Chunk::Leaf { .. } => panic!("page encoder must emit a FORM"),
    }
    iff::emit(&file)
}

/// The bundled spec with a `FREE` chunk inserted at the root, so a run has
/// several components to walk and one rewrite to apply.
fn bundle_with_free_chunk() -> Vec<u8> {
    let bytes = fs::read("tests/fixtures/DjVu3Spec_bundled.djvu").unwrap();
    let mut file = iff::parse(&bytes).unwrap();
    match &mut file.root {
        Chunk::Form { children, .. } => children.insert(
            2,
            Chunk::Leaf {
                id: *b"FREE",
                data: vec![0; 33],
            },
        ),
        Chunk::Leaf { .. } => panic!("bundle must be a FORM"),
    }
    iff::emit(&file)
}

/// Collect every event a run reports, in order.
fn recording_optimizer(
    request: OptimizationRequest,
) -> (Optimizer, Arc<Mutex<Vec<ProgressEvent>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    let optimizer = Optimizer::new(request)
        .with_progress(move |event| sink.lock().unwrap().push(event.clone()));
    (optimizer, events)
}

/// Every event of `phase`, checked for one-step indices and monotone bytes.
fn phase_events(events: &[ProgressEvent], phase: OptimizationPhase) -> Vec<ProgressEvent> {
    let phase_events: Vec<ProgressEvent> = events
        .iter()
        .filter(|event| event.phase == phase)
        .cloned()
        .collect();
    for (position, event) in phase_events.iter().enumerate() {
        assert_eq!(event.component_index, position, "{phase:?} index order");
        assert_eq!(event.component_count, phase_events.len(), "{phase:?} count");
        if position > 0 {
            assert!(
                event.bytes_so_far >= phase_events[position - 1].bytes_so_far,
                "{phase:?} bytes must not decrease"
            );
        }
    }
    phase_events
}

#[test]
fn progress_reports_plan_then_rewrite_then_verify_per_component() {
    let input = bundle_with_free_chunk();
    let (optimizer, events) = recording_optimizer(OptimizationRequest::lossless_cleanup());

    let result = optimizer.optimize(&input).unwrap();
    let events = events.lock().unwrap().clone();

    // Phases arrive in order and never interleave.
    let order: Vec<OptimizationPhase> = events.iter().map(|event| event.phase).collect();
    let mut sorted = order.clone();
    sorted.sort_by_key(|phase| match phase {
        OptimizationPhase::Plan => 0,
        OptimizationPhase::Rewrite => 1,
        OptimizationPhase::Verify => 2,
        _ => 3,
    });
    assert_eq!(order, sorted, "phases must not interleave");

    // Plan: one event per root child of the DJVM, the FREE chunk included.
    let plan = phase_events(&events, OptimizationPhase::Plan);
    let root_children = match iff::parse(&input).unwrap().root {
        Chunk::Form { children, .. } => children.len(),
        Chunk::Leaf { .. } => unreachable!(),
    };
    assert_eq!(plan.len(), root_children);
    assert_eq!(plan[0].component_id, *b"DIRM");
    assert_eq!(plan[2].component_id, *b"FREE");
    assert!(plan.iter().any(|event| event.component_id == *b"DJVU"));
    // The header-inclusive sizes add up to the root payload minus its
    // secondary ID, short of one pad byte per odd-length child.
    let walked = plan.last().unwrap().bytes_so_far;
    let payload = input.len() - 16;
    assert!(
        walked <= payload && walked + root_children >= payload,
        "plan walked {walked} B of a {payload} B payload"
    );

    // Rewrite: exactly the one FREE removal, its input payload as bytes.
    let rewrite = phase_events(&events, OptimizationPhase::Rewrite);
    assert_eq!(rewrite.len(), 1);
    assert_eq!(rewrite[0].component_id, *b"FREE");
    assert_eq!(rewrite[0].bytes_so_far, 33);

    // Verify: the output's components, one fewer than the input's.
    let verify = phase_events(&events, OptimizationPhase::Verify);
    assert_eq!(verify.len(), root_children - 1);
    assert!(verify.iter().all(|event| event.component_id != *b"FREE"));
    let walked = verify.last().unwrap().bytes_so_far;
    let payload = result.bytes.len() - 16;
    assert!(
        walked <= payload && walked + root_children >= payload,
        "verify walked {walked} B of a {payload} B payload"
    );
}

#[test]
fn plan_alone_reports_only_the_plan_phase() {
    let input = page_with_free_and_unknown_chunk();
    let (optimizer, events) = recording_optimizer(OptimizationRequest::lossless_cleanup());

    optimizer.plan(&input).unwrap();
    let events = events.lock().unwrap().clone();

    // A single-page DJVU is one component: the root FORM itself.
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].phase, OptimizationPhase::Plan);
    assert_eq!(events[0].component_id, *b"DJVU");
    assert_eq!(events[0].component_count, 1);
    assert_eq!(events[0].bytes_so_far, input.len() - 4);
}

#[test]
fn pass_through_run_reports_no_rewrite_events() {
    let input = PageEncoder::from_bitmap(&Bitmap::new(8, 8))
        .encode()
        .unwrap();
    let (optimizer, events) = recording_optimizer(OptimizationRequest::lossless_cleanup());

    optimizer.optimize(&input).unwrap();
    let events = events.lock().unwrap().clone();

    let phases: Vec<OptimizationPhase> = events.iter().map(|event| event.phase).collect();
    assert_eq!(phases, [OptimizationPhase::Plan, OptimizationPhase::Verify]);
}

#[test]
fn optimizer_without_hook_is_unchanged() {
    let input = bundle_with_free_chunk();
    let silent = Optimizer::new(OptimizationRequest::lossless_cleanup());
    let (observed, _) = recording_optimizer(OptimizationRequest::lossless_cleanup());
    assert_eq!(
        silent.optimize(&input).unwrap(),
        observed.optimize(&input).unwrap()
    );
}

/// An optimizer that records events and cancels once `after` of them have
/// been reported; the hook is polled before the next component.
fn cancelling_after(after: usize) -> (Optimizer, Arc<Mutex<Vec<ProgressEvent>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::new(AtomicUsize::new(0));
    let sink = Arc::clone(&events);
    let counter = Arc::clone(&seen);
    let optimizer = Optimizer::new(OptimizationRequest::lossless_cleanup())
        .with_progress(move |event| {
            sink.lock().unwrap().push(event.clone());
            counter.fetch_add(1, Ordering::SeqCst);
        })
        .with_cancel(move || seen.load(Ordering::SeqCst) >= after);
    (optimizer, events)
}

#[test]
fn cancel_before_start_reports_nothing() {
    let input = bundle_with_free_chunk();
    let (optimizer, events) = cancelling_after(0);
    assert!(matches!(
        optimizer.plan(&input),
        Err(OptimizeError::Cancelled)
    ));
    assert!(matches!(
        optimizer.optimize(&input),
        Err(OptimizeError::Cancelled)
    ));
    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn cancel_stops_at_a_component_boundary_in_plan() {
    let input = bundle_with_free_chunk();
    let (optimizer, events) = cancelling_after(3);
    assert!(matches!(
        optimizer.optimize(&input),
        Err(OptimizeError::Cancelled)
    ));
    let events = events.lock().unwrap().clone();
    // Exactly the three components handled before the poll saw the stop.
    assert_eq!(events.len(), 3);
    assert!(
        events
            .iter()
            .all(|event| event.phase == OptimizationPhase::Plan)
    );
    assert_eq!(events[2].component_index, 2);
}

#[test]
fn cancel_after_rewrite_withholds_output_and_skips_verify() {
    let input = bundle_with_free_chunk();
    let root_children = match iff::parse(&input).unwrap().root {
        Chunk::Form { children, .. } => children.len(),
        Chunk::Leaf { .. } => unreachable!(),
    };
    // Stop once the single rewrite has been reported: the next poll is the
    // one ahead of verification.
    let (optimizer, events) = cancelling_after(root_children + 1);
    assert!(matches!(
        optimizer.optimize(&input),
        Err(OptimizeError::Cancelled)
    ));
    let events = events.lock().unwrap().clone();
    assert_eq!(events.len(), root_children + 1);
    assert_eq!(events.last().unwrap().phase, OptimizationPhase::Rewrite);
    assert!(
        events
            .iter()
            .all(|event| event.phase != OptimizationPhase::Verify)
    );
}

#[test]
fn cancel_hook_that_never_fires_changes_nothing() {
    let input = bundle_with_free_chunk();
    let plain = Optimizer::new(OptimizationRequest::lossless_cleanup());
    let (polite, _) = cancelling_after(usize::MAX);
    assert_eq!(
        plain.optimize(&input).unwrap(),
        polite.optimize(&input).unwrap()
    );
}

#[test]
fn several_free_chunks_in_one_parent_are_all_removed_in_plan_order() {
    // Two FREE leaves ahead of the Xtra chunk and one after it: removing the
    // earlier ones shifts the later paths, which the optimizer must track.
    let bitmap = Bitmap::new(8, 8);
    let encoded = PageEncoder::from_bitmap(&bitmap).encode().unwrap();
    let mut file = iff::parse(&encoded).unwrap();
    let free = |len: usize| Chunk::Leaf {
        id: *b"FREE",
        data: vec![0; len],
    };
    match &mut file.root {
        Chunk::Form { children, .. } => {
            children.insert(1, free(5));
            children.insert(2, free(6));
            children.push(Chunk::Leaf {
                id: *b"Xtra",
                data: b"preserve me".to_vec(),
            });
            children.push(free(7));
        }
        Chunk::Leaf { .. } => unreachable!(),
    }
    let input = iff::emit(&file);
    let (optimizer, events) = recording_optimizer(OptimizationRequest::lossless_cleanup());

    let result = optimizer.optimize(&input).unwrap();

    let rewrites = &result.report.rewritten_components;
    assert_eq!(rewrites.len(), 3);
    assert_eq!(
        rewrites
            .iter()
            .map(|item| item.input_bytes)
            .collect::<Vec<_>>(),
        [5, 6, 7]
    );
    let output = iff::parse(&result.bytes).unwrap();
    let ids: Vec<[u8; 4]> = match output.root {
        Chunk::Form { children, .. } => children
            .iter()
            .map(|chunk| match chunk {
                Chunk::Leaf { id, .. } => *id,
                Chunk::Form { secondary_id, .. } => *secondary_id,
            })
            .collect(),
        Chunk::Leaf { .. } => unreachable!(),
    };
    assert!(!ids.contains(b"FREE"));
    assert!(ids.contains(b"Xtra"));
    // Rewrite events come in plan order with increasing bytes.
    let rewrite = phase_events(&events.lock().unwrap(), OptimizationPhase::Rewrite);
    assert_eq!(
        rewrite
            .iter()
            .map(|event| event.bytes_so_far)
            .collect::<Vec<_>>(),
        [5, 11, 18]
    );
}

#[test]
fn lossless_cleanup_removes_free_but_preserves_pixels_and_unknown_chunks() {
    let input = page_with_free_and_unknown_chunk();
    let optimizer = Optimizer::new(OptimizationRequest::lossless_cleanup());

    let plan = optimizer.plan(&input).unwrap();
    assert_eq!(plan.preset, OptimizationPreset::LosslessCleanup);
    assert!(plan.changed);
    assert_eq!(plan.rewritten_components.len(), 1);
    assert_eq!(plan.rewritten_components[0].chunk_id, *b"FREE");
    assert_eq!(plan.rewritten_components[0].input_bytes, 17);
    assert_eq!(plan.rewritten_components[0].output_bytes, 0);

    let result = optimizer.optimize(&input).unwrap();
    assert!(result.report.changed);
    assert_eq!(result.report.input_bytes, input.len());
    assert_eq!(result.report.output_bytes, result.bytes.len());
    assert!(result.bytes.len() < input.len());

    let document = djvu_rs::DjVuDocument::parse(&result.bytes).unwrap();
    let page = document.page(0).unwrap();
    assert!(page.raw_chunk(b"FREE").is_none());
    assert_eq!(page.raw_chunk(b"Xtra"), Some(&b"preserve me"[..]));
    assert_eq!(page.extract_mask().unwrap().unwrap().width, 8);
}

#[test]
fn already_clean_input_is_byte_identical_and_reported_as_pass_through() {
    let bitmap = Bitmap::new(8, 8);
    let input = PageEncoder::from_bitmap(&bitmap).encode().unwrap();
    let optimizer = Optimizer::new(OptimizationRequest::lossless_cleanup());

    let plan = optimizer.plan(&input).unwrap();
    assert!(!plan.changed);
    assert!(plan.rewritten_components.is_empty());
    assert!(plan.warnings.is_empty());

    let result = optimizer.optimize(&input).unwrap();
    assert_eq!(result.bytes, input);
    assert!(!result.report.changed);
    assert!(result.report.rewritten_components.is_empty());
}

#[test]
fn archival_request_is_typed_and_reports_unmet_target_without_lossy_reencode() {
    let input = page_with_free_and_unknown_chunk();
    let request = OptimizationRequest::archival().with_target_size(1);
    let optimizer = Optimizer::new(request);

    let plan = optimizer.plan(&input).unwrap();
    assert_eq!(plan.preset, OptimizationPreset::Archival);
    assert!(!plan.target_met);
    assert!(
        plan.warnings
            .iter()
            .any(|warning| warning.contains("target size"))
    );

    let result = optimizer.optimize(&input).unwrap();
    assert_eq!(result.bytes.len(), plan.output_bytes);
    assert!(
        result
            .report
            .warnings
            .iter()
            .any(|warning| warning.contains("archival"))
    );
}

#[test]
fn max_ssim_loss_warns_and_does_not_pretend_to_gate_lossless_cleanup() {
    let input = page_with_free_and_unknown_chunk();
    let request = OptimizationRequest::lossless_cleanup().with_max_ssim_loss(0.001);
    let optimizer = Optimizer::new(request);

    let plan = optimizer.plan(&input).unwrap();
    assert!(plan.quality_floor_met);
    assert!(
        plan.warnings
            .iter()
            .any(|warning| warning.contains("does not measure SSIM")),
        "expected SSIM-reservation warning, got {:?}",
        plan.warnings
    );
}

#[test]
fn plan_and_report_have_machine_readable_json() {
    let input = page_with_free_and_unknown_chunk();
    let optimizer = Optimizer::new(OptimizationRequest::lossless_cleanup());
    let plan = optimizer.plan(&input).unwrap();
    let result = optimizer.optimize(&input).unwrap();

    let plan_json: serde_json::Value = serde_json::from_str(&plan.to_json()).unwrap();
    let report_json: serde_json::Value = serde_json::from_str(&result.report.to_json()).unwrap();
    assert_eq!(plan_json["preset"], "lossless-cleanup");
    assert_eq!(plan_json["changed"], true);
    assert_eq!(plan_json["rewritten_components"][0]["chunk_id"], "FREE");
    assert_eq!(report_json["output_bytes"], result.bytes.len());
}

#[test]
fn cli_dry_run_does_not_write_and_normal_run_is_atomic() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.djvu");
    let output = dir.path().join("output.djvu");
    fs::write(&input, page_with_free_and_unknown_chunk()).unwrap();

    let dry_run = Command::cargo_bin("djvu")
        .unwrap()
        .args([
            "optimize",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--preset",
            "lossless-cleanup",
            "--dry-run",
        ])
        .assert()
        .success()
        // stderr is a pipe here, so no progress line and no escape codes.
        .stderr(predicates::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let dry_json: serde_json::Value = serde_json::from_slice(&dry_run).unwrap();
    assert_eq!(dry_json["changed"], true);
    assert!(!output.exists());

    Command::cargo_bin("djvu")
        .unwrap()
        .args([
            "optimize",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--preset",
            "lossless-cleanup",
        ])
        .assert()
        .success()
        .stderr(predicates::str::is_empty());
    assert!(output.exists());
    assert_eq!(
        fs::read(&input).unwrap(),
        page_with_free_and_unknown_chunk()
    );
    djvu_rs::DjVuDocument::parse(&fs::read(output).unwrap()).unwrap();

    Command::cargo_bin("djvu")
        .unwrap()
        .args([
            "optimize",
            input.to_str().unwrap(),
            "--output",
            input.to_str().unwrap(),
            "--preset",
            "lossless-cleanup",
        ])
        .assert()
        .failure();
}
