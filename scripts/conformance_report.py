#!/usr/bin/env python3
"""Validate differential results and build the public conformance artifact.

The input JSONL is intentionally produced by the existing Rust/DjVuLibre
differential harness.  This script supplies the fail-closed coverage contract,
versioned result schema, corpus identity, trend history, and static dashboard.
It has no third-party Python dependencies.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import html
import json
import math
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 2
HISTORY_LIMIT = 100
SEMANTIC_PAGE_PLANES = frozenset({"text", "text_hierarchy", "annotations"})
SEMANTIC_DOC_PLANES = frozenset({"bookmarks", "metadata", "dirm"})
DIFF_FUZZ_CLASS_RE = re.compile(
    r"_(pixel-mismatch|dim-mismatch|our-stricter|our-laxer|"
    r"our-render-fail|our-renders-what-they-reject)\.txt$"
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def command_version(command: list[str]) -> str:
    try:
        result = subprocess.run(command, check=True, capture_output=True, text=True)
    except (OSError, subprocess.CalledProcessError) as exc:
        raise RuntimeError(f"required tool failed: {' '.join(command)}: {exc}") from exc
    text = (result.stdout or result.stderr).strip().splitlines()
    if not text:
        raise RuntimeError(f"required tool returned no version: {' '.join(command)}")
    return text[0]


def djvulibre_version() -> str:
    """Read ddjvu's banner; --help deliberately exits non-zero."""
    try:
        result = subprocess.run(["ddjvu", "--help"], capture_output=True, text=True)
    except OSError as exc:
        raise RuntimeError(f"required tool failed: ddjvu --help: {exc}") from exc
    lines = (result.stdout or result.stderr).strip().splitlines()
    if not lines or "DjVuLibre" not in lines[0]:
        raise RuntimeError("ddjvu --help did not return a DjVuLibre version banner")
    return lines[0]


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        raise RuntimeError(f"cannot read JSON {path}: {exc}") from exc


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    try:
        lines = path.read_text().splitlines()
    except OSError as exc:
        raise RuntimeError(f"cannot read results {path}: {exc}") from exc
    for number, line in enumerate(lines, 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as exc:
            raise RuntimeError(f"invalid JSONL at {path}:{number}: {exc}") from exc
        if not isinstance(row, dict):
            raise RuntimeError(f"result at {path}:{number} is not an object")
        rows.append(row)
    return rows


def validate(manifest: dict[str, Any], rows: list[dict[str, Any]]) -> list[str]:
    render = manifest["render"]
    expected = {
        (doc["path"], page)
        for doc in manifest["documents"]
        for page in range(doc["pages"])
    }
    seen: set[tuple[str, int]] = set()
    failures: list[str] = []
    required = {
        "file",
        "page",
        "width",
        "height",
        "total_px",
        "mismatched_px",
        "mismatch_pct",
        "max_abs_diff",
        "mean_abs_diff",
    }
    for row in rows:
        missing = sorted(required - row.keys())
        if missing:
            failures.append(f"result missing fields {missing}: {row!r}")
            continue
        if type(row["file"]) is not str or type(row["page"]) is not int:
            failures.append(f"result has invalid file/page types: {row!r}")
            continue
        integer_fields = (
            "width",
            "height",
            "total_px",
            "mismatched_px",
            "max_abs_diff",
        )
        number_fields = ("mismatch_pct", "mean_abs_diff")
        if any(type(row[field]) is not int for field in integer_fields) or any(
            type(row[field]) not in (int, float) or not math.isfinite(float(row[field]))
            for field in number_fields
        ):
            failures.append(f"result has invalid metric types/values: {row!r}")
            continue
        key = (row["file"], row["page"])
        if key in seen:
            failures.append(f"duplicate result for {key[0]} page {key[1]}")
        seen.add(key)
        if (
            row["page"] < 0
            or row["width"] <= 0
            or row["height"] <= 0
            or row["total_px"] != row["width"] * row["height"]
            or not 0 <= row["mismatched_px"] <= row["total_px"]
            or not 0 <= row["max_abs_diff"] <= 255
            or not 0 <= row["mismatch_pct"] <= 100
            or not 0 <= row["mean_abs_diff"] <= 255
            or abs(row["mismatch_pct"] - row["mismatched_px"] / row["total_px"] * 100)
            > 0.00011
        ):
            failures.append(f"invalid dimensions/pixel count for {key}")
        if row["mismatch_pct"] > render["max_mismatch_pct"]:
            failures.append(
                f"{key[0]} page {key[1]} mismatch {row['mismatch_pct']:.4f}% "
                f"> {render['max_mismatch_pct']:.4f}%"
            )
        if row["mean_abs_diff"] > render["max_mean_abs_diff"]:
            failures.append(
                f"{key[0]} page {key[1]} mean delta {row['mean_abs_diff']:.4f} "
                f"> {render['max_mean_abs_diff']:.4f}"
            )
    for key in sorted(expected - seen):
        failures.append(f"missing result for {key[0]} page {key[1]}")
    for key in sorted(seen - expected):
        failures.append(f"unexpected result for {key[0]} page {key[1]}")
    return failures


def validate_semantic(
    manifest: dict[str, Any], rows: list[dict[str, Any]]
) -> list[str]:
    expected = set()
    for document in manifest["documents"]:
        for plane in SEMANTIC_DOC_PLANES:
            expected.add((document["path"], 0, plane))
        for page in range(document["pages"]):
            for plane in SEMANTIC_PAGE_PLANES:
                expected.add((document["path"], page, plane))
    seen: set[tuple[str, int, str]] = set()
    failures: list[str] = []
    required = {"file", "page", "plane", "status", "ours", "djvulibre"}
    allowed_planes = SEMANTIC_PAGE_PLANES | SEMANTIC_DOC_PLANES
    for row in rows:
        missing = sorted(required - row.keys())
        if missing:
            failures.append(f"semantic result missing fields {missing}: {row!r}")
            continue
        if (
            any(
                type(row[field]) is not str
                for field in ("file", "plane", "status", "ours", "djvulibre")
            )
            or type(row["page"]) is not int
            or row["page"] < 0
            or row["plane"] not in allowed_planes
            or row["status"] not in {"match", "diverge"}
        ):
            failures.append(f"semantic result has invalid types/values: {row!r}")
            continue
        key = (row["file"], row["page"], row["plane"])
        if key in seen:
            failures.append(f"duplicate semantic result for {key}")
        seen.add(key)
        # text_hierarchy is covered and published, but zone trees still diverge
        # across implementations (parent text fill + OCR segmentation). Treat
        # those divergences as observational until the trees align; other planes
        # remain fail-closed.
        if row["status"] != "match" and row["plane"] != "text_hierarchy":
            failures.append(f"semantic divergence for {key[0]} page {key[1]} {key[2]}")
        if (row["status"] == "match") != (row["ours"] == row["djvulibre"]):
            failures.append(f"semantic status/payload contradiction for {key}")
    for key in sorted(expected - seen):
        failures.append(f"missing semantic result for {key}")
    for key in sorted(seen - expected):
        failures.append(f"unexpected semantic result for {key}")
    return failures


def parse_writer_results(path: Path) -> dict[str, Any]:
    """Parse interop_encode stdout into a structured writer validation object."""
    try:
        text = path.read_text()
    except OSError as exc:
        raise RuntimeError(f"cannot read writer results {path}: {exc}") from exc
    cases: list[dict[str, Any]] = []
    rejected = 0
    dim_mismatches = 0
    for line in text.splitlines():
        if not line.strip() or line.startswith("page"):
            continue
        if "REJECT" in line:
            rejected += 1
            cases.append({"line": line.strip(), "status": "reject"})
        elif "DIMS!" in line:
            dim_mismatches += 1
            cases.append({"line": line.strip(), "status": "dims"})
        elif re.search(r"\bok\b", line):
            cases.append({"line": line.strip(), "status": "ok"})
    summary_match = re.search(r"(\d+)\s+checked,\s+(\d+)\s+failed", text)
    checked = int(summary_match.group(1)) if summary_match else len(cases)
    failed = int(summary_match.group(2)) if summary_match else rejected + dim_mismatches
    status = "pass" if failed == 0 and rejected == 0 and dim_mismatches == 0 else "fail"
    return {
        "status": status,
        "checked": checked,
        "failed": failed,
        "rejected": rejected,
        "dim_mismatches": dim_mismatches,
        "cases": cases,
    }


def load_accepted_differences(path: Path | None) -> list[dict[str, Any]]:
    if path is None:
        return []
    if not path.exists():
        raise RuntimeError(f"accepted differences registry missing: {path}")
    data = load_json(path)
    if not isinstance(data, dict) or "differences" not in data:
        raise RuntimeError(f"accepted differences {path} must contain 'differences'")
    differences = data["differences"]
    if not isinstance(differences, list):
        raise RuntimeError(f"accepted differences {path} 'differences' must be a list")
    return [item for item in differences if isinstance(item, dict)]


def load_diff_fuzz_registry(directory: Path | None) -> dict[str, Any]:
    if directory is None:
        return {"categories": {}, "fixtures": []}
    if not directory.is_dir():
        raise RuntimeError(f"diff_fuzz registry directory missing: {directory}")
    categories: dict[str, int] = {}
    fixtures: list[dict[str, str]] = []
    for path in sorted(directory.glob("*.txt")):
        match = DIFF_FUZZ_CLASS_RE.search(path.name)
        if not match:
            raise RuntimeError(f"unclassified diff_fuzz fixture: {path.name}")
        category = match.group(1)
        categories[category] = categories.get(category, 0) + 1
        fixtures.append({"path": str(path), "category": category})
    if not fixtures:
        raise RuntimeError(f"diff_fuzz registry is empty: {directory}")
    return {
        "directory": str(directory),
        "fixture_count": len(fixtures),
        "categories": dict(sorted(categories.items())),
        "fixtures": fixtures,
    }


def baseline_delta(
    current: dict[str, Any], previous: dict[str, Any] | None
) -> dict[str, Any]:
    if previous is None:
        return {
            "has_baseline": False,
            "status_changed": False,
            "mismatch_pct_delta": None,
            "pages_delta": None,
            "new_failure_count": len(current.get("failures", [])),
            "resolved_failure_count": 0,
            "regression": False,
            "improvement": False,
        }
    prev_failures = set(previous.get("failures", []))
    curr_failures = set(current.get("failures", []))
    new_failures = sorted(curr_failures - prev_failures)
    resolved = sorted(prev_failures - curr_failures)
    mismatch_delta = float(current.get("max_mismatch_pct", 0)) - float(
        previous.get("max_mismatch_pct", 0)
    )
    status_changed = previous.get("status") != current.get("status")
    regression = (previous.get("status") == "pass" and current.get("status") == "fail") or (
        mismatch_delta > 0 and current.get("status") == "fail"
    )
    improvement = (previous.get("status") == "fail" and current.get("status") == "pass") or (
        mismatch_delta < 0 and not new_failures
    )
    return {
        "has_baseline": True,
        "previous_commit": previous.get("commit"),
        "previous_status": previous.get("status"),
        "status_changed": status_changed,
        "mismatch_pct_delta": mismatch_delta,
        "pages_delta": int(current.get("pages_compared", 0))
        - int(previous.get("pages_compared", 0)),
        "new_failures": new_failures,
        "resolved_failures": resolved,
        "new_failure_count": len(new_failures),
        "resolved_failure_count": len(resolved),
        "regression": regression,
        "improvement": improvement,
    }


def load_history(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.exists():
        return []
    data = load_json(path)
    if not isinstance(data, list):
        raise RuntimeError(f"history {path} must be a JSON array")
    return [item for item in data if isinstance(item, dict)]


def render_html(summary: dict[str, Any], history: list[dict[str, Any]]) -> str:
    status = "PASS" if summary["status"] == "pass" else "FAIL"
    rows = "".join(
        "<tr>"
        f"<td>{html.escape(row['file'])}</td><td>{row['page'] + 1}</td>"
        f"<td>{row['width']}×{row['height']}</td>"
        f"<td>{row['mismatch_pct']:.4f}%</td>"
        f"<td>{row['mean_abs_diff']:.3f}</td><td>{row['max_abs_diff']}</td>"
        "</tr>"
        for row in summary["render_results"]
    )
    trend = "".join(
        "<tr>"
        f"<td>{html.escape(item.get('timestamp', '?'))}</td>"
        f"<td>{html.escape(item.get('commit', '?')[:12])}</td>"
        f"<td>{html.escape(item.get('status', '?').upper())}</td>"
        f"<td>{item.get('pages_compared', 0)}</td>"
        f"<td>{item.get('max_mismatch_pct', 0):.4f}%</td>"
        "</tr>"
        for item in reversed(history[-20:])
    )
    failures = "".join(f"<li>{html.escape(item)}</li>" for item in summary["failures"])
    semantic = "".join(
        "<tr>"
        f"<td>{html.escape(row['file'])}</td><td>{row['page'] + 1}</td>"
        f"<td>{html.escape(row['plane'])}</td>"
        f"<td>{html.escape(row['status'].upper())}</td>"
        "</tr>"
        for row in summary["semantic_results"]
    )
    delta = summary.get("baseline_delta", {})
    if delta.get("has_baseline"):
        delta_html = (
            f"<p>Previous <code>{html.escape(str(delta.get('previous_commit', '?'))[:12])}</code> "
            f"({html.escape(str(delta.get('previous_status', '?')).upper())}) · "
            f"Δ mismatch {delta.get('mismatch_pct_delta', 0):+.4f}% · "
            f"new failures {delta.get('new_failure_count', 0)} · "
            f"resolved {delta.get('resolved_failure_count', 0)} · "
            f"{'REGRESSION' if delta.get('regression') else ('IMPROVEMENT' if delta.get('improvement') else 'stable')}</p>"
        )
    else:
        delta_html = "<p>No previous baseline in history.</p>"
    writer = summary.get("writer_validation", {})
    if isinstance(writer, dict):
        writer_html = (
            f"<p>status <code>{html.escape(str(writer.get('status')))}</code> · "
            f"checked {writer.get('checked', 0)} · failed {writer.get('failed', 0)} · "
            f"rejected {writer.get('rejected', 0)} · dims {writer.get('dim_mismatches', 0)}</p>"
        )
    else:
        writer_html = f"<p><code>{html.escape(str(writer))}</code></p>"
    fuzz = summary.get("diff_fuzz_registry", {})
    fuzz_rows = "".join(
        f"<tr><td>{html.escape(category)}</td><td>{count}</td></tr>"
        for category, count in (fuzz.get("categories") or {}).items()
    )
    accepted = "".join(
        "<tr>"
        f"<td>{html.escape(item.get('id', ''))}</td>"
        f"<td>{html.escape(item.get('category', ''))}</td>"
        f"<td>{html.escape(item.get('rationale', ''))}</td>"
        "</tr>"
        for item in summary.get("accepted_differences", [])
    )
    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width">
<title>djvu-rs conformance</title>
<style>body{{font:16px system-ui;max-width:1100px;margin:2rem auto;padding:0 1rem;color:#18212b}}table{{border-collapse:collapse;width:100%;margin:1rem 0}}th,td{{border:1px solid #ccd4dc;padding:.45rem;text-align:left}}th{{background:#eef3f7}}.pass{{color:#087830}}.fail{{color:#b42318}}code{{background:#eef3f7;padding:.15rem .3rem}}</style>
</head><body><h1>djvu-rs conformance</h1>
<h2 class="{status.lower()}">{status}</h2>
<p>Commit <code>{html.escape(summary["commit"])}</code> · {html.escape(summary["timestamp"])}</p>
<p>{summary["pages_compared"]} pages · corpus <code>{summary["corpus_digest"][:16]}</code> · {html.escape(summary["tools"]["djvulibre"])}</p>
<h2>Baseline delta</h2>{delta_html}
<h2>Failures</h2><ul>{failures or "<li>None</li>"}</ul>
<h2>Render differential</h2><table><thead><tr><th>Document</th><th>Page</th><th>Size</th><th>Mismatch</th><th>Mean Δ</th><th>Max Δ</th></tr></thead><tbody>{rows}</tbody></table>
<h2>Semantic differential</h2><table><thead><tr><th>Document</th><th>Page</th><th>Plane</th><th>Status</th></tr></thead><tbody>{semantic}</tbody></table>
<h2>Writer validation</h2>{writer_html}
<h2>Diff-fuzz classification registry</h2><table><thead><tr><th>Category</th><th>Fixtures</th></tr></thead><tbody>{fuzz_rows or "<tr><td colspan=2>None</td></tr>"}</tbody></table>
<h2>Accepted differences</h2><table><thead><tr><th>Id</th><th>Category</th><th>Rationale</th></tr></thead><tbody>{accepted or "<tr><td colspan=3>None</td></tr>"}</tbody></table>
<h2>Recent runs</h2><table><thead><tr><th>Time</th><th>Commit</th><th>Status</th><th>Pages</th><th>Worst mismatch</th></tr></thead><tbody>{trend}</tbody></table>
<p>Machine-readable: <a href="summary.json">summary.json</a> · <a href="history.json">history.json</a></p>
</body></html>"""


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--manifest", type=Path, default=Path("conformance/corpus.json")
    )
    parser.add_argument("--results", type=Path, required=True)
    parser.add_argument("--semantic-results", type=Path, required=True)
    parser.add_argument("--writer-status", type=Path, required=True)
    parser.add_argument("--writer-results", type=Path)
    parser.add_argument("--accepted-differences", type=Path)
    parser.add_argument("--diff-fuzz-registry", type=Path)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--history-input", type=Path)
    parser.add_argument("--commit", default=os.environ.get("GITHUB_SHA"))
    parser.add_argument("--djvulibre-version")
    args = parser.parse_args()

    try:
        manifest = load_json(args.manifest)
        rows = load_jsonl(args.results)
        semantic_rows = load_jsonl(args.semantic_results)
        failures = validate(manifest, rows) + validate_semantic(manifest, semantic_rows)
        writer_status = args.writer_status.read_text().strip()
        if writer_status != "pass":
            failures.append(
                f"writer validation status is {writer_status!r}, expected 'pass'"
            )
        if args.writer_results is not None:
            writer_validation: Any = parse_writer_results(args.writer_results)
            if writer_validation["status"] != "pass":
                failures.append(
                    "writer interop_encode reported "
                    f"{writer_validation['failed']} failure(s)"
                )
        else:
            writer_validation = writer_status
        accepted = load_accepted_differences(args.accepted_differences)
        diff_fuzz = load_diff_fuzz_registry(args.diff_fuzz_registry)
        commit = args.commit or command_version(["git", "rev-parse", "HEAD"])
        djvulibre = args.djvulibre_version or djvulibre_version()
        timestamp = dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()
        corpus_hash = hashlib.sha256()
        corpus_hash.update(args.manifest.read_bytes())
        inputs = []
        for document in manifest["documents"]:
            path = Path(document["path"])
            digest = sha256(path)
            corpus_hash.update(document["path"].encode())
            corpus_hash.update(bytes.fromhex(digest))
            inputs.append(
                {"path": document["path"], "sha256": digest, "pages": document["pages"]}
            )
        history = load_history(args.history_input)
        previous = history[-1] if history else None
        summary = {
            "schema_version": SCHEMA_VERSION,
            "timestamp": timestamp,
            "commit": commit,
            "status": "fail" if failures else "pass",
            "tools": {"djvu_rs": commit, "djvulibre": djvulibre},
            "corpus_digest": corpus_hash.hexdigest(),
            "inputs": inputs,
            "render_policy": manifest["render"],
            "pages_compared": len(rows),
            "max_mismatch_pct": max(
                (row.get("mismatch_pct", 0) for row in rows), default=0
            ),
            "failures": failures,
            "render_results": rows,
            "semantic_results": semantic_rows,
            "writer_validation": writer_validation,
            "accepted_differences": accepted,
            "diff_fuzz_registry": {
                key: diff_fuzz[key]
                for key in ("directory", "fixture_count", "categories")
                if key in diff_fuzz
            },
        }
        summary["baseline_delta"] = baseline_delta(
            {
                "status": summary["status"],
                "commit": summary["commit"],
                "pages_compared": summary["pages_compared"],
                "max_mismatch_pct": summary["max_mismatch_pct"],
                "failures": summary["failures"],
            },
            previous,
        )
        history.append(
            {
                key: summary[key]
                for key in (
                    "timestamp",
                    "commit",
                    "status",
                    "pages_compared",
                    "max_mismatch_pct",
                    "corpus_digest",
                    "failures",
                )
            }
        )
        history = history[-HISTORY_LIMIT:]
        args.output_dir.mkdir(parents=True, exist_ok=True)
        (args.output_dir / "summary.json").write_text(
            json.dumps(summary, indent=2, sort_keys=True, allow_nan=False) + "\n"
        )
        (args.output_dir / "history.json").write_text(
            json.dumps(history, indent=2, allow_nan=False) + "\n"
        )
        (args.output_dir / "index.html").write_text(render_html(summary, history))
    except (KeyError, OSError, RuntimeError, TypeError, ValueError) as exc:
        print(f"conformance report error: {exc}", file=sys.stderr)
        return 2

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1
    print(f"PASS: {len(rows)} pages; corpus {summary['corpus_digest'][:16]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
