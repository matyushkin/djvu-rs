"""Run every ```python block of README.md and GUIDE.md against the built module.

The Python counterpart of the Rust README/guide doctests: a documented example
that no longer runs fails here instead of on a reader's machine. Each block
runs in its own temporary directory where `scan.djvu` and `book.djvu` are a
bundled multi-page fixture with bookmarks, so the export and editing examples
work on real files.
"""

from __future__ import annotations

import re
import shutil
import warnings
from pathlib import Path

import pytest

from conftest import FIXTURES_DIR

DJVU_PY = Path(__file__).resolve().parents[1]
DOCS = ("README.md", "GUIDE.md")
BLOCK = re.compile(r"^```python\n(.*?)^```", re.MULTILINE | re.DOTALL)


def _blocks():
    for name in DOCS:
        text = (DJVU_PY / name).read_text(encoding="utf-8")
        for match in BLOCK.finditer(text):
            line = text.count("\n", 0, match.start()) + 2
            yield pytest.param(name, line, match.group(1), id=f"{name}:{line}")


def test_docs_have_examples():
    assert len(list(_blocks())) >= 5


@pytest.mark.parametrize(("doc", "line", "source"), list(_blocks()))
def test_doc_example_runs(doc, line, source, tmp_path, monkeypatch):
    fixture = FIXTURES_DIR / "navm_fgbz.djvu"
    if not fixture.exists():
        pytest.skip(f"fixture not found: {fixture}")
    for name in ("scan.djvu", "book.djvu"):
        shutil.copy(fixture, tmp_path / name)
    monkeypatch.chdir(tmp_path)

    # Blocks that continue an earlier one rely on its `import djvu_rs as djvu`.
    if not source.startswith("from __future__"):
        source = "import djvu_rs as djvu\n" + source
        line -= 1
    code = compile("\n" * (line - 1) + source, str(DJVU_PY / doc), "exec")
    with warnings.catch_warnings():
        # A documented example must not use a deprecated API.
        warnings.simplefilter("error", DeprecationWarning)
        exec(code, {"__name__": "__doc_example__"})
