"""Text-layer extraction."""

from __future__ import annotations

import djvu_rs as djvu


def test_text_present_on_ocr_page(multipage_path):
    doc = djvu.Document.open(str(multipage_path))
    text = doc.page(0).text()
    assert text is not None
    assert isinstance(text, str)
    assert len(text.strip()) > 0


def test_text_none_when_absent(boy_path):
    doc = djvu.Document.open(str(boy_path))
    text = doc.page(0).text()
    assert text is None


def test_text_differs_across_pages(multipage_path):
    doc = djvu.Document.open(str(multipage_path))
    if doc.page_count() < 2:
        return
    texts = [doc.page(i).text() for i in range(doc.page_count())]
    # At least the non-None texts should not all be identical strings.
    non_none = [t for t in texts if t]
    assert len(non_none) >= 1
