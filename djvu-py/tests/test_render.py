"""Render → pixel sanity: dimensions, non-blank content, DPI scaling."""

from __future__ import annotations

import djvu_rs as djvu


def _is_blank(data: bytes) -> bool:
    """True if every pixel is identical (e.g. all-white or all-zero)."""
    return len(set(data)) <= 1


def test_render_native_dpi_dimensions(boy_path):
    doc = djvu.Document.open(str(boy_path))
    page = doc.page(0)
    pix = page.render()
    assert pix.width == page.width
    assert pix.height == page.height


def test_render_data_length_matches_rgba(boy_path):
    doc = djvu.Document.open(str(boy_path))
    pix = doc.page(0).render()
    assert len(pix.data()) == pix.width * pix.height * 4


def test_render_not_blank(boy_path):
    doc = djvu.Document.open(str(boy_path))
    pix = doc.page(0).render()
    assert not _is_blank(pix.data())


def test_render_jb2_not_blank(boy_jb2_path):
    doc = djvu.Document.open(str(boy_jb2_path))
    pix = doc.page(0).render()
    assert not _is_blank(pix.data())
    # Bilevel JB2 content should render as more than just two literal bytes
    # (RGBA channels vary even for a black/white page).
    assert len(set(pix.data())) > 1


def test_render_with_target_dpi_scales(boy_path):
    doc = djvu.Document.open(str(boy_path))
    page = doc.page(0)
    native = page.render()
    half = page.render(dpi=page.dpi / 2)
    assert half.width == round(native.width / 2)
    assert half.height == round(native.height / 2)


def test_render_upscale(boy_path):
    doc = djvu.Document.open(str(boy_path))
    page = doc.page(0)
    native = page.render()
    doubled = page.render(dpi=page.dpi * 2)
    assert doubled.width == native.width * 2
    assert doubled.height == native.height * 2


def test_render_all_pages_multipage(multipage_path):
    doc = djvu.Document.open(str(multipage_path))
    for i in range(doc.page_count()):
        page = doc.page(i)
        pix = page.render()
        assert pix.width == page.width
        assert pix.height == page.height
        assert not _is_blank(pix.data())


def test_render_alpha_channel_opaque(boy_path):
    """RGBA output — alpha channel should be fully opaque (255) for a plain page."""
    doc = djvu.Document.open(str(boy_path))
    pix = doc.page(0).render()
    data = pix.data()
    alphas = data[3::4]
    assert set(alphas) == {255}
