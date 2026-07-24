#!/usr/bin/env python3
"""Smoke-test an *installed* djvu-rs wheel (not a repo editable build).

Verifies import, open/from_bytes, render, and text extraction against a fixture.
Intended to run after `pip install path/to/djvu_rs-*.whl` from a clean cwd.
"""

from __future__ import annotations

import argparse
import os
import sys
import tempfile
from pathlib import Path


def _assert_not_loading_repo_sources(module_file: str, repo_root: Path | None) -> None:
    if repo_root is None:
        return
    resolved = Path(module_file).resolve()
    djvu_py = (repo_root / "djvu-py").resolve()
    if resolved.is_relative_to(djvu_py):
        raise AssertionError(
            f"djvu_rs loaded from repository sources ({resolved}); "
            "smoke test must use an installed wheel"
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--fixture",
        type=Path,
        required=True,
        help="Path to a .djvu fixture (copied into a temp dir before open)",
    )
    parser.add_argument(
        "--expect-version",
        default=None,
        help="If set, require djvu_rs.__version__ to equal this string",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=None,
        help="Optional repo root; used to reject loading djvu-py sources",
    )
    args = parser.parse_args()

    fixture = args.fixture.resolve()
    if not fixture.is_file():
        print(f"fixture not found: {fixture}", file=sys.stderr)
        return 2

    # Import only after argv parse so failures are actionable.
    try:
        import djvu_rs as djvu
    except Exception as exc:  # noqa: BLE001 — surface any import failure
        print(f"import djvu_rs failed: {exc}", file=sys.stderr)
        return 1

    module_file = getattr(djvu, "__file__", None)
    if module_file:
        print(f"djvu_rs.__file__ = {module_file}")
        try:
            _assert_not_loading_repo_sources(module_file, args.repo_root)
        except AssertionError as exc:
            print(exc, file=sys.stderr)
            return 1

    version = getattr(djvu, "__version__", None)
    print(f"djvu_rs.__version__ = {version}")
    if args.expect_version is not None and version != args.expect_version:
        print(
            f"version mismatch: got {version!r}, expected {args.expect_version!r}",
            file=sys.stderr,
        )
        return 1

    for name in ("Error", "DecodeError", "IoError", "PageIndexError"):
        if not hasattr(djvu, name):
            print(f"missing typed exception class: {name}", file=sys.stderr)
            return 1

    with tempfile.TemporaryDirectory(prefix="djvu-wheel-smoke-") as tmp:
        local_fixture = Path(tmp) / fixture.name
        local_fixture.write_bytes(fixture.read_bytes())

        # Run with cwd outside any checkout so relative imports cannot sneak in.
        os.chdir(tmp)

        try:
            doc = djvu.Document.open(str(local_fixture))
        except Exception as exc:  # noqa: BLE001
            print(f"Document.open failed: {exc}", file=sys.stderr)
            return 1

        if doc.page_count() < 1:
            print("page_count() < 1", file=sys.stderr)
            return 1

        page = doc.page(0)
        try:
            pixmap = page.render(dpi=72)
        except Exception as exc:  # noqa: BLE001
            print(f"page.render failed: {exc}", file=sys.stderr)
            return 1

        if pixmap.width < 1 or pixmap.height < 1:
            print("render produced empty pixmap", file=sys.stderr)
            return 1
        if len(pixmap.data()) != pixmap.width * pixmap.height * 4:
            print("RGBA buffer length mismatch", file=sys.stderr)
            return 1

        try:
            _text = page.text()
        except Exception as exc:  # noqa: BLE001
            print(f"page.text failed: {exc}", file=sys.stderr)
            return 1

        # Typed error smoke: garbage input must raise DecodeError.
        try:
            djvu.Document.from_bytes(b"not-a-djvu")
        except djvu.DecodeError:
            pass
        except Exception as exc:  # noqa: BLE001
            print(f"expected DecodeError, got {type(exc).__name__}: {exc}", file=sys.stderr)
            return 1
        else:
            print("expected DecodeError for garbage bytes", file=sys.stderr)
            return 1

        try:
            doc.page(doc.page_count())
        except djvu.PageIndexError:
            pass
        except Exception as exc:  # noqa: BLE001
            print(
                f"expected PageIndexError, got {type(exc).__name__}: {exc}",
                file=sys.stderr,
            )
            return 1
        else:
            print("expected PageIndexError for out-of-range page", file=sys.stderr)
            return 1

    print(
        f"python wheel smoke OK: pages={doc.page_count()} "
        f"render={pixmap.width}x{pixmap.height}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
