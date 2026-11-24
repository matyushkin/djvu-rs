"""djvu_rs.encode_page: one image in, a single-page DjVu file out."""

from __future__ import annotations

import shutil
import subprocess

import numpy as np
import pytest
from PIL import Image

import djvu_rs as djvu

WIDTH, HEIGHT = 96, 64


def bilevel_rgba() -> np.ndarray:
    """A pure black-and-white pattern: a frame, a bar and a few dots."""
    img = np.full((HEIGHT, WIDTH, 4), 255, dtype=np.uint8)
    ink = np.zeros((HEIGHT, WIDTH), dtype=bool)
    ink[4:6, 4:-4] = ink[-6:-4, 4:-4] = True
    ink[4:-4, 4:6] = ink[4:-4, -6:-4] = True
    ink[20:40, 30:36] = True
    ink[12:50:7, 50:80:5] = True
    img[ink, :3] = 0
    return img


def colour_rgba() -> np.ndarray:
    """A smooth colour gradient: continuous-tone content."""
    y, x = np.mgrid[0:HEIGHT, 0:WIDTH]
    img = np.empty((HEIGHT, WIDTH, 4), dtype=np.uint8)
    img[..., 0] = x * 255 // (WIDTH - 1)
    img[..., 1] = y * 255 // (HEIGHT - 1)
    img[..., 2] = (x + y) * 255 // (WIDTH + HEIGHT - 2)
    img[..., 3] = 255
    return img


def decode(data: bytes) -> np.ndarray:
    page = djvu.Document.from_bytes(data).page(0)
    return page.render().to_numpy()


def test_lossless_round_trip_is_pixel_exact():
    src = bilevel_rgba()
    data = djvu.encode_page(src, quality="lossless")
    assert isinstance(data, bytes)
    assert np.array_equal(decode(data), src)


@pytest.mark.skipif(shutil.which("ddjvu") is None, reason="DjVuLibre ddjvu not installed")
def test_lossless_round_trip_with_djvulibre(tmp_path):
    src = bilevel_rgba()
    path = tmp_path / "page.djvu"
    path.write_bytes(djvu.encode_page(src, quality="lossless"))
    out = tmp_path / "page.ppm"
    subprocess.run(["ddjvu", "-format=ppm", str(path), str(out)], check=True)
    decoded = np.asarray(Image.open(out).convert("RGB"))
    assert np.array_equal(decoded, src[..., :3])


def test_every_input_kind_gives_the_same_file():
    src = bilevel_rgba()
    expected = djvu.encode_page(src, quality="lossless")
    pixmap = djvu.Document.from_bytes(expected).page(0).render()
    inputs = {
        "rgb array": src[..., :3],
        "gray array": src[..., 0],
        "PIL RGBA": Image.fromarray(src, "RGBA"),
        "PIL L": Image.fromarray(src[..., 0], "L"),
        "PIL 1": Image.fromarray(src[..., 0], "L").convert("1"),
        "Pixmap": pixmap,
    }
    for name, image in inputs.items():
        assert djvu.encode_page(image, quality="lossless") == expected, name
    raw = src.tobytes()
    assert djvu.encode_page(raw, size=(WIDTH, HEIGHT), quality="lossless") == expected
    assert (
        djvu.encode_page(bytearray(raw), size=(WIDTH, HEIGHT), quality="lossless")
        == expected
    )


def test_auto_picks_lossless_for_black_and_white():
    src = bilevel_rgba()
    assert djvu.encode_page(src) == djvu.encode_page(src, quality="lossless")


def document_rgba() -> np.ndarray:
    """Black ink on cream paper: a layered document page."""
    img = bilevel_rgba()
    img[img[..., 0] == 255, :3] = (240, 232, 210)
    return img


@pytest.mark.parametrize(
    ("quality", "source"),
    [
        ("auto", colour_rgba),
        ("photo", colour_rgba),
        ("auto", document_rgba),
        ("quality", document_rgba),
        ("archival", document_rgba),
    ],
)
def test_colour_profiles_keep_size_and_content(quality, source):
    src = source()
    out = decode(djvu.encode_page(src, quality=quality))
    assert out.shape == src.shape
    error = np.abs(out[..., :3].astype(int) - src[..., :3].astype(int)).mean()
    assert error < 4, error


def test_dpi_is_stored():
    data = djvu.encode_page(bilevel_rgba(), dpi=600)
    assert djvu.Document.from_bytes(data).page(0).dpi == 600
    assert djvu.Document.from_bytes(djvu.encode_page(bilevel_rgba())).page(0).dpi == 300


def test_text_layer_and_metadata_round_trip():
    layer = {
        "text": "frame bar",
        "zones": [
            {
                "kind": "Page",
                "rect": {"x": 0, "y": 0, "width": WIDTH, "height": HEIGHT},
                "text": "frame bar",
                "children": [
                    {
                        "kind": "Word",
                        "rect": {"x": 4, "y": 4, "width": 40, "height": 10},
                        "text": "frame",
                    },
                    {
                        "kind": "Word",
                        "rect": {"x": 30, "y": 20, "width": 6, "height": 20},
                        "text": "bar",
                    },
                ],
            }
        ],
    }
    data = djvu.encode_page(
        bilevel_rgba(), text_layer=layer, metadata={"title": "Pattern"}
    )
    doc = djvu.Document.from_bytes(data)
    stored = doc.page(0).text_layer()
    assert stored["text"] == "frame bar"
    words = stored["zones"][0]["children"]
    assert [w["text"] for w in words] == ["frame", "bar"]
    assert [w["rect"] for w in words] == [w["rect"] for w in layer["zones"][0]["children"]]
    assert doc.metadata()["title"] == "Pattern"


def test_an_encoded_page_can_be_merged():
    page = djvu.encode_page(bilevel_rgba())
    assert djvu.Document.from_bytes(djvu.merge([page, page])).page_count() == 2


def test_bad_arguments():
    src = bilevel_rgba()
    with pytest.raises(ValueError, match="quality"):
        djvu.encode_page(src, quality="best")
    with pytest.raises(ValueError, match="dpi"):
        djvu.encode_page(src, dpi=0)
    with pytest.raises(ValueError, match="do not match"):
        djvu.encode_page(src.tobytes(), size=(WIDTH, HEIGHT + 1))
    with pytest.raises(ValueError, match="shape"):
        djvu.encode_page(np.zeros((2, 3, 2), dtype=np.uint8))
    with pytest.raises(ValueError, match="empty"):
        djvu.encode_page(b"", size=(0, 0))
    with pytest.raises(ValueError, match="text layer"):
        djvu.encode_page(src, text_layer={"zones": 1})
    with pytest.raises(TypeError, match="uint8"):
        djvu.encode_page(np.zeros((4, 4), dtype=np.float32))
    with pytest.raises(TypeError, match="size"):
        djvu.encode_page(src.tobytes())
    with pytest.raises(TypeError, match="Pixmap"):
        djvu.encode_page(42)


def test_an_image_too_large_for_the_format_raises_encode_error():
    wide = np.full((1, 70_000), 255, dtype=np.uint8)
    with pytest.raises(djvu.EncodeError):
        djvu.encode_page(wide, quality="lossless")
    assert issubclass(djvu.EncodeError, djvu.Error)
