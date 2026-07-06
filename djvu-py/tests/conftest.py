"""Shared pytest fixtures for the djvu-py test suite.

Corpus/fixture files live at the workspace root (``tests/corpus`` and
``tests/fixtures``), one level above the ``djvu-py`` crate — resolved
relative to this file so pytest can be invoked from any working directory.
"""

from __future__ import annotations

from pathlib import Path

import pytest

# djvu-py/tests/conftest.py -> djvu-py/ -> repo root
REPO_ROOT = Path(__file__).resolve().parents[2]
CORPUS_DIR = REPO_ROOT / "tests" / "corpus"
FIXTURES_DIR = REPO_ROOT / "tests" / "fixtures"


def _require(path: Path) -> Path:
    if not path.exists():
        pytest.skip(f"corpus file not found: {path}")
    return path


@pytest.fixture(scope="session")
def boy_path() -> Path:
    """Smallest fixture: 1 page, 192x256, no text layer."""
    return _require(FIXTURES_DIR / "boy.djvu")


@pytest.fixture(scope="session")
def boy_jb2_path() -> Path:
    """1 page, JB2 (bilevel) encoded, no text layer."""
    return _require(FIXTURES_DIR / "boy_jb2.djvu")


@pytest.fixture(scope="session")
def multipage_path() -> Path:
    """2 pages, has an OCR text layer — small enough to be fast."""
    return _require(CORPUS_DIR / "cable_1973_100133.djvu")


@pytest.fixture(scope="session")
def watchmaker_path() -> Path:
    """12 pages, ~2550x3301 each — used for GIL-release timing."""
    return _require(CORPUS_DIR / "watchmaker.djvu")


@pytest.fixture(scope="session")
def boy_bytes(boy_path: Path) -> bytes:
    return boy_path.read_bytes()
