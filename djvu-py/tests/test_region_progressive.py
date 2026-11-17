"""#583: region render == crop of full render; coarse/progressive semantics.

All through the one `Page.render(dpi, size=, region=, quality=)` call; the
older `render_region` / `render_coarse` / `render_progressive` were removed in
1.0.0.
"""
import djvu_rs
import pytest

from conftest import FIXTURES_DIR as FIXTURES


@pytest.fixture()
def color_doc():
    return djvu_rs.Document.open(str(FIXTURES / "colorbook.djvu"))


def crop(pm, x, y, w, h):
    data = pm.data()
    stride = pm.width * 4
    return b"".join(
        data[(y + row) * stride + x * 4:(y + row) * stride + (x + w) * 4] for row in range(h)
    )


def test_region_matches_crop_of_full_render(color_doc):
    page = color_doc.page(0)
    full = page.render()
    x, y, w, h = 40, 60, 128, 96
    region = page.render(region=(x, y, w, h))
    assert (region.width, region.height) == (w, h)
    assert region.data() == crop(full, x, y, w, h)


def test_region_at_size_matches_crop_of_render_at_size(color_doc):
    page = color_doc.page(0)
    size = (page.width // 2, page.height // 2)
    full = page.render(size=size)
    assert (full.width, full.height) == size
    x, y, w, h = 10, 20, 64, 48
    assert page.render(size=size, region=(x, y, w, h)).data() == crop(full, x, y, w, h)


def test_render_coarse_and_progressive(color_doc):
    page = color_doc.page(0)
    n = page.bg44_chunk_count
    assert n >= 1
    coarse = page.render(dpi=100, quality="coarse")
    assert coarse.width > 0
    # At native resolution the last progressive stage is byte-identical to the
    # full render; at downscaled DPI the progressive compositor path may take
    # a different (equally valid) resampling route, so only shape is checked.
    last = page.render(quality=n - 1)
    full = page.render(quality="full")
    assert last.data() == full.data(), "last progressive stage must equal the full render"
    scaled = page.render(dpi=100, quality=n - 1)
    at_100 = page.render(dpi=100)
    assert (scaled.width, scaled.height) == (at_100.width, at_100.height)


def test_render_rejects_bad_arguments(color_doc):
    page = color_doc.page(0)
    with pytest.raises(ValueError):
        page.render(dpi=100, size=(10, 10))
    with pytest.raises(ValueError):
        page.render(quality="sharp")
    with pytest.raises(djvu_rs.DecodeError):
        page.render(quality=page.bg44_chunk_count)


def test_coarse_render_of_bilevel_page_raises():
    doc = djvu_rs.Document.open(str(FIXTURES / "boy_jb2.djvu"))
    with pytest.raises(djvu_rs.DecodeError):
        doc.page(0).render(quality="coarse")


def test_removed_variants_are_gone(color_doc):
    page = color_doc.page(0)
    for name in ("render_region", "render_coarse", "render_progressive"):
        assert not hasattr(page, name), name
