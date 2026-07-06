"""Document open/from_bytes, page metadata, and error paths."""

from __future__ import annotations

import pytest

import djvu_rs as djvu


def test_open_from_path(boy_path):
    doc = djvu.Document.open(str(boy_path))
    assert doc.page_count() == 1


def test_open_missing_path_raises():
    with pytest.raises(Exception):
        djvu.Document.open("/nonexistent/path/does-not-exist.djvu")


def test_from_bytes(boy_bytes):
    doc = djvu.Document.from_bytes(boy_bytes)
    assert doc.page_count() == 1


def test_open_and_from_bytes_agree(boy_path, boy_bytes):
    doc_a = djvu.Document.open(str(boy_path))
    doc_b = djvu.Document.from_bytes(boy_bytes)
    assert doc_a.page_count() == doc_b.page_count()
    pa, pb = doc_a.page(0), doc_b.page(0)
    assert (pa.width, pa.height, pa.dpi) == (pb.width, pb.height, pb.dpi)


def test_multipage_page_count(multipage_path):
    doc = djvu.Document.open(str(multipage_path))
    assert doc.page_count() == 2


def test_page_metadata(boy_path):
    doc = djvu.Document.open(str(boy_path))
    page = doc.page(0)
    assert page.width == 192
    assert page.height == 256
    assert page.dpi == 100


def test_jb2_page_metadata(boy_jb2_path):
    doc = djvu.Document.open(str(boy_jb2_path))
    page = doc.page(0)
    assert page.width == 192
    assert page.height == 256
    assert page.dpi == 300


def test_page_index_out_of_range_raises(boy_path):
    doc = djvu.Document.open(str(boy_path))
    with pytest.raises(IndexError):
        doc.page(doc.page_count())


def test_page_index_far_out_of_range_raises(boy_path):
    doc = djvu.Document.open(str(boy_path))
    with pytest.raises(IndexError):
        doc.page(999)


# ── Error paths: bad input must raise, never crash the interpreter ──────────


@pytest.mark.parametrize(
    "data",
    [
        b"",
        b"not a djvu file",
        b"AT&T" + b"\x00" * 8,  # looks IFF-ish but truncated/invalid
        bytes(range(256)),
    ],
)
def test_from_bytes_garbage_raises(data):
    with pytest.raises(Exception):
        djvu.Document.from_bytes(data)


def test_open_directory_raises(tmp_path):
    with pytest.raises(Exception):
        djvu.Document.open(str(tmp_path))


def test_from_bytes_truncated_valid_file_raises(boy_bytes):
    truncated = boy_bytes[: len(boy_bytes) // 2]
    with pytest.raises(Exception):
        djvu.Document.from_bytes(truncated)
