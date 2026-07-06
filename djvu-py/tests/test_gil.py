"""GIL-release check for round-41's `py.detach(...)` calls.

Rendering releases the GIL for the decode/composite/resample work, so two
threads rendering different pages should run measurably faster than the same
work done sequentially. This is a timing-based test, so it uses a lenient
threshold and is skipped outright on single-CPU runners or when the
measurement is too noisy to trust — it reports rather than flakes CI.
"""

from __future__ import annotations

import os
import threading
import time

import pytest

import djvu_rs as djvu

# Lenient on purpose: real speedup on a multi-core machine is close to 2x,
# but CI runners are noisy/shared, so we only assert *some* parallelism.
SPEEDUP_THRESHOLD = 1.3


def _cpu_count() -> int:
    return os.cpu_count() or 1


@pytest.mark.skipif(_cpu_count() < 2, reason="GIL-release speedup needs >=2 CPUs")
def test_render_releases_gil(watchmaker_path):
    doc = djvu.Document.open(str(watchmaker_path))
    n_pages = min(doc.page_count(), 6)
    assert n_pages >= 2

    def render(i: int) -> None:
        doc.page(i).render()

    # Warm up (page metadata lookups, allocator, etc.) so the timed runs
    # measure steady-state render cost only.
    render(0)

    t0 = time.perf_counter()
    for i in range(n_pages):
        render(i)
    sequential = time.perf_counter() - t0

    t0 = time.perf_counter()
    threads = [threading.Thread(target=render, args=(i,)) for i in range(n_pages)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    parallel = time.perf_counter() - t0

    speedup = sequential / parallel if parallel > 0 else float("inf")

    if sequential < 0.02:
        pytest.skip(f"render too fast to measure reliably ({sequential:.4f}s sequential)")

    # Non-fatal report: print unconditionally for visibility in CI logs, then
    # do a lenient assert. If this proves flaky in practice, downgrade the
    # assert to a warning — the measurement/print stays either way.
    print(f"\n[gil] sequential={sequential:.4f}s parallel={parallel:.4f}s speedup={speedup:.2f}x")
    assert speedup > SPEEDUP_THRESHOLD, (
        f"expected some GIL-release parallelism (>{SPEEDUP_THRESHOLD}x), got {speedup:.2f}x "
        f"(sequential={sequential:.4f}s, parallel={parallel:.4f}s) — "
        "this is a timing-sensitive CI check, rerun before treating as a regression"
    )
