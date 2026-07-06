"""Round-41 (PY_ZEROCOPY) coverage: buffer protocol, zero-copy accessors,
and Pixmap lifetime safety when a view outlives the Python-side reference.
"""

from __future__ import annotations

import gc

import numpy as np
import pytest
from PIL import Image

import djvu_rs as djvu


@pytest.fixture()
def pixmap(boy_path):
    doc = djvu.Document.open(str(boy_path))
    return doc.page(0).render()


# ── memoryview / buffer protocol ────────────────────────────────────────────


def test_memoryview_basic(pixmap):
    mv = memoryview(pixmap)
    assert mv.format == "B"
    assert mv.readonly is True
    assert mv.ndim == 1
    assert len(mv) == pixmap.width * pixmap.height * 4


def test_memoryview_bytes_equal_data(pixmap):
    mv = memoryview(pixmap)
    assert bytes(mv) == pixmap.data()


def test_bytes_of_pixmap_equal_data(pixmap):
    assert bytes(pixmap) == pixmap.data()


def test_memoryview_is_readonly_raises_on_write(pixmap):
    mv = memoryview(pixmap)
    with pytest.raises(TypeError):
        mv[0] = 0


# ── numpy.frombuffer round-trip vs .data() byte-equality ────────────────────


def test_numpy_frombuffer_roundtrip_matches_data(pixmap):
    arr = np.frombuffer(pixmap, dtype=np.uint8)
    assert arr.tobytes() == pixmap.data()


def test_to_numpy_matches_data(pixmap):
    arr = pixmap.to_numpy()
    assert arr.shape == (pixmap.height, pixmap.width, 4)
    assert arr.dtype == np.uint8
    assert arr.tobytes() == pixmap.data()


# ── zero-copy vs copy variants ──────────────────────────────────────────────


def test_to_numpy_zerocopy_matches_copy(pixmap):
    zc = pixmap.to_numpy_zerocopy()
    copy = pixmap.to_numpy()
    assert zc.shape == copy.shape
    assert zc.dtype == copy.dtype
    assert np.array_equal(zc, copy)


def test_to_numpy_zerocopy_shares_memory_with_pixmap(pixmap):
    zc = pixmap.to_numpy_zerocopy()
    # Mutate isn't possible (read-only), but we can confirm the base buffer's
    # bytes match .data() exactly, i.e. it's a view over the same allocation.
    assert zc.tobytes() == pixmap.data()
    assert zc.flags["WRITEABLE"] is False


def test_to_pil_zerocopy_matches_copy(pixmap):
    zc = pixmap.to_pil_zerocopy()
    copy = pixmap.to_pil()
    assert zc.size == copy.size
    assert zc.mode == copy.mode == "RGBA"
    assert zc.tobytes() == copy.tobytes()


def test_to_pil_and_to_numpy_agree(pixmap):
    pil_bytes = pixmap.to_pil().tobytes()
    numpy_bytes = pixmap.to_numpy().tobytes()
    assert pil_bytes == numpy_bytes == pixmap.data()


# ── lifetime safety: view outlives the Python `Pixmap` reference ───────────


def test_memoryview_outlives_pixmap_del(boy_path):
    doc = djvu.Document.open(str(boy_path))
    pix = doc.page(0).render()
    expected = pix.data()
    mv = memoryview(pix)

    del pix
    gc.collect()

    # The buffer keeps the underlying Pixmap allocation alive via the
    # Py_buffer.obj reference — data must still be valid and correct.
    assert bytes(mv) == expected

    # Releasing the view is what finally frees the Pixmap.
    mv.release()


def test_numpy_zerocopy_outlives_pixmap_del(boy_path):
    doc = djvu.Document.open(str(boy_path))
    pix = doc.page(0).render()
    expected = pix.data()
    arr = pix.to_numpy_zerocopy()

    del pix
    gc.collect()

    assert arr.tobytes() == expected


def test_pil_zerocopy_outlives_pixmap_del(boy_path):
    doc = djvu.Document.open(str(boy_path))
    pix = doc.page(0).render()
    expected = pix.data()
    img = pix.to_pil_zerocopy()

    del pix
    gc.collect()

    assert img.tobytes() == expected


def test_many_pixmaps_clean_free_no_leak_crash(boy_path):
    """Allocate and drop many Pixmaps/views — exercises __getbuffer__ /
    __releasebuffer__ repeatedly; must not crash or hang."""
    doc = djvu.Document.open(str(boy_path))
    page = doc.page(0)
    for _ in range(50):
        pix = page.render()
        mv = memoryview(pix)
        _ = bytes(mv)
        mv.release()
        del pix
    gc.collect()
