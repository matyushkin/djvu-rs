"""#583: region render == crop of full render; coarse/progressive semantics."""
import djvu_rs
import pytest

from conftest import FIXTURES_DIR as FIXTURES


@pytest.fixture()
def color_doc():
    return djvu_rs.Document.open(str(FIXTURES / "colorbook.djvu"))


def test_region_matches_crop_of_full_render(color_doc):
    page = color_doc.page(0)
    full = page.render()
    x, y, w, h = 40, 60, 128, 96
    region = page.render_region(x, y, w, h)
    assert (region.width, region.height) == (w, h)
    full_bytes = full.data()
    region_bytes = region.data()
    stride = full.width * 4
    for row in range(h):
        a = full_bytes[(y + row) * stride + x * 4:(y + row) * stride + (x + w) * 4]
        b = region_bytes[row * w * 4:(row + 1) * w * 4]
        assert a == b, f"row {row} differs"


def test_render_coarse_and_progressive(color_doc):
    page = color_doc.page(0)
    n = page.bg44_chunk_count
    assert n >= 1
    coarse = page.render_coarse(dpi=100)
    assert coarse is not None and coarse.width > 0
    # At native resolution the last progressive stage is byte-identical to the
    # full render; at downscaled DPI the progressive compositor path may take
    # a different (equally valid) resampling route, so only shape is checked.
    last = page.render_progressive(n - 1)
    full = page.render()
    assert last.data() == full.data(), "last progressive stage must equal the full render"
    scaled = page.render_progressive(n - 1, dpi=100)
    assert (scaled.width, scaled.height) == (page.render(dpi=100).width, page.render(dpi=100).height)


def test_render_coarse_none_for_bilevel():
    doc = djvu_rs.Document.open(str(FIXTURES / "boy_jb2.djvu"))
    assert doc.page(0).render_coarse() is None
