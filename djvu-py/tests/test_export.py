"""Export to PDF, EPUB, CBZ and TIFF.

Every format has two entry points: ``to_x()`` returns the bytes and
``write_x(path)`` streams the same output into a file. The tests below check
the magic bytes, check that the two forms agree, and check that the options
that change the output really change it.

Byte-for-byte equality between the two forms is asserted on purpose. The
in-memory form wraps the streaming one in the Rust crate, so a difference
would mean the file path took a different route.
"""

from __future__ import annotations

import zipfile

import pytest

import djvu_rs as djvu


@pytest.fixture(scope="module")
def doc(multipage_path):
    """A two-page document with a text layer — every exporter has work to do."""
    return djvu.Document.open(str(multipage_path))


# ---- PDF ---------------------------------------------------------------------


def test_to_pdf_produces_a_pdf(doc):
    data = doc.to_pdf(dpi=72)
    assert data.startswith(b"%PDF-")
    assert data.rstrip().endswith(b"%%EOF")


def test_write_pdf_matches_to_pdf(doc, tmp_path):
    out = tmp_path / "out.pdf"
    doc.write_pdf(str(out), dpi=72)
    assert out.read_bytes() == doc.to_pdf(dpi=72)


def test_pdf_lossless_differs_from_jpeg(doc):
    lossy = doc.to_pdf(dpi=72, jpeg_quality=40)
    lossless = doc.to_pdf(dpi=72, jpeg_quality=None)
    assert lossy != lossless


def test_pdf_dpi_changes_the_size(doc):
    small = doc.to_pdf(dpi=50)
    large = doc.to_pdf(dpi=150)
    assert len(large) > len(small)


def test_write_pdf_to_a_bad_path_raises(doc):
    with pytest.raises(djvu.IoError):
        doc.write_pdf("/nonexistent/directory/out.pdf")


# ---- EPUB --------------------------------------------------------------------


def test_to_epub_produces_a_zip(doc):
    data = doc.to_epub(dpi=72)
    assert data[:2] == b"PK"


def test_write_epub_matches_to_epub(doc, tmp_path):
    out = tmp_path / "out.epub"
    doc.write_epub(str(out), dpi=72, modified="2026-01-01T00:00:00Z")
    expected = doc.to_epub(dpi=72, modified="2026-01-01T00:00:00Z")
    assert out.read_bytes() == expected


def test_epub_title_reaches_the_metadata(doc, tmp_path):
    out = tmp_path / "titled.epub"
    doc.write_epub(str(out), dpi=72, title="A Borrowed Name", author="Nobody")
    with zipfile.ZipFile(out) as z:
        opf = next(n for n in z.namelist() if n.endswith(".opf"))
        text = z.read(opf).decode("utf-8")
    assert "A Borrowed Name" in text
    assert "Nobody" in text


# ---- CBZ ---------------------------------------------------------------------


def test_to_cbz_holds_one_png_per_page(doc, tmp_path):
    out = tmp_path / "out.cbz"
    doc.write_cbz(str(out), dpi=72)
    with zipfile.ZipFile(out) as z:
        names = z.namelist()
        assert len(names) == doc.page_count()
        assert names == sorted(names)
        assert z.read(names[0])[:8] == b"\x89PNG\r\n\x1a\n"


def test_write_cbz_matches_to_cbz(doc, tmp_path):
    out = tmp_path / "out.cbz"
    doc.write_cbz(str(out), dpi=72)
    assert out.read_bytes() == doc.to_cbz(dpi=72)


def test_cbz_page_selection(doc):
    with zipfile.ZipFile(_as_file(doc.to_cbz(dpi=72, pages=[1]))) as z:
        assert len(z.namelist()) == 1


def test_cbz_rotation_swaps_the_sides(doc, tmp_path):
    upright = tmp_path / "upright.cbz"
    turned = tmp_path / "turned.cbz"
    doc.write_cbz(str(upright), dpi=72)
    doc.write_cbz(str(turned), dpi=72, rotation=90)
    assert upright.read_bytes() != turned.read_bytes()


def test_cbz_rejects_a_diagonal_rotation(doc):
    with pytest.raises(ValueError):
        doc.to_cbz(rotation=45)


# ---- TIFF --------------------------------------------------------------------


def test_to_tiff_produces_a_tiff(doc):
    data = doc.to_tiff(scale=0.25)
    assert data[:4] in (b"II*\x00", b"MM\x00*")


def test_write_tiff_matches_to_tiff(doc, tmp_path):
    out = tmp_path / "out.tiff"
    doc.write_tiff(str(out), scale=0.25)
    assert out.read_bytes() == doc.to_tiff(scale=0.25)


def test_tiff_bilevel_differs_from_colour(doc):
    colour = doc.to_tiff(scale=0.25, mode="color")
    bilevel = doc.to_tiff(scale=0.25, mode="bilevel")
    assert colour != bilevel


def test_tiff_rejects_an_unknown_mode(doc):
    with pytest.raises(ValueError):
        doc.to_tiff(mode="sepia")


def test_tiff_rejects_an_unknown_compression(doc):
    with pytest.raises(ValueError):
        doc.to_tiff(mode="bilevel", bilevel_compression="lzw")


# ---- Shared ------------------------------------------------------------------


def test_export_errors_are_catchable_as_the_base_error(doc):
    assert issubclass(djvu.ExportError, djvu.Error)


def _as_file(data: bytes):
    """Wrap bytes so zipfile can read them without a temporary file."""
    import io

    return io.BytesIO(data)
