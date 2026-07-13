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
import subprocess
import sys
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1
HISTORY_LIMIT = 100


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
        expected.add((document["path"], 0, "bookmarks"))
        for page in range(document["pages"]):
            expected.add((document["path"], page, "text"))
            expected.add((document["path"], page, "annotations"))
    seen: set[tuple[str, int, str]] = set()
    failures: list[str] = []
    required = {"file", "page", "plane", "status", "ours", "djvulibre"}
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
            or row["plane"] not in {"text", "annotations", "bookmarks"}
            or row["status"] not in {"match", "diverge"}
        ):
            failures.append(f"semantic result has invalid types/values: {row!r}")
            continue
        key = (row["file"], row["page"], row["plane"])
        if key in seen:
            failures.append(f"duplicate semantic result for {key}")
        seen.add(key)
        if row["status"] != "match":
            failures.append(f"semantic divergence for {key[0]} page {key[1]} {key[2]}")
        if (row["status"] == "match") != (row["ours"] == row["djvulibre"]):
            failures.append(f"semantic status/payload contradiction for {key}")
    for key in sorted(expected - seen):
        failures.append(f"missing semantic result for {key}")
    for key in sorted(seen - expected):
        failures.append(f"unexpected semantic result for {key}")
    return failures


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
    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width">
<title>djvu-rs conformance</title>
<style>body{{font:16px system-ui;max-width:1100px;margin:2rem auto;padding:0 1rem;color:#18212b}}table{{border-collapse:collapse;width:100%;margin:1rem 0}}th,td{{border:1px solid #ccd4dc;padding:.45rem;text-align:left}}th{{background:#eef3f7}}.pass{{color:#087830}}.fail{{color:#b42318}}code{{background:#eef3f7;padding:.15rem .3rem}}</style>
</head><body><h1>djvu-rs conformance</h1>
<h2 class="{status.lower()}">{status}</h2>
<p>Commit <code>{html.escape(summary["commit"])}</code> · {html.escape(summary["timestamp"])}</p>
<p>{summary["pages_compared"]} pages · corpus <code>{summary["corpus_digest"][:16]}</code> · {html.escape(summary["tools"]["djvulibre"])}</p>
<h2>Failures</h2><ul>{failures or "<li>None</li>"}</ul>
<h2>Render differential</h2><table><thead><tr><th>Document</th><th>Page</th><th>Size</th><th>Mismatch</th><th>Mean Δ</th><th>Max Δ</th></tr></thead><tbody>{rows}</tbody></table>
<h2>Semantic differential</h2><table><thead><tr><th>Document</th><th>Page</th><th>Plane</th><th>Status</th></tr></thead><tbody>{semantic}</tbody></table>
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
            "writer_validation": writer_status,
        }
        history = load_history(args.history_input)
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
