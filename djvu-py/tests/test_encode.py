"""djvu_rs.encode_page: one image in, a single-page DjVu file out."""

from __future__ import annotations

import shutil
import subprocess

import numpy as np
import pytest
from PIL import Image, ImageDraw

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


# ---- encode_document ---------------------------------------------------------


def text_page(number: int) -> np.ndarray:
    """A black-and-white page: the same frame on every page plus a page mark."""
    img = bilevel_rgba()
    img[52:58, 10 : 10 + 6 * number, :3] = 0
    return img


def document_like_rgba() -> np.ndarray:
    """Dark text bars over tinted paper."""
    img = np.empty((HEIGHT, WIDTH, 4), dtype=np.uint8)
    img[...] = (236, 228, 205, 255)
    img[10:14, 8:-8, :3] = img[24:28, 8:-20, :3] = img[38:42, 8:-30, :3] = 20
    return img


def render_all(data: bytes) -> list[np.ndarray]:
    doc = djvu.Document.from_bytes(data)
    return [doc.page(i).render().to_numpy() for i in range(doc.page_count())]


def test_document_lossless_is_pixel_exact():
    pages = [text_page(n) for n in (1, 2, 3)]
    data = djvu.encode_document(pages, quality="lossless", dpi=200)
    assert isinstance(data, bytes)
    doc = djvu.Document.from_bytes(data)
    assert doc.page_count() == 3
    assert doc.page(0).dpi == 200
    for rendered, src in zip(render_all(data), pages):
        assert np.array_equal(rendered, src)


@pytest.mark.skipif(shutil.which("ddjvu") is None, reason="DjVuLibre ddjvu not installed")
def test_document_lossless_with_djvulibre(tmp_path):
    pages = [text_page(n) for n in (1, 2, 3)]
    path = tmp_path / "book.djvu"
    path.write_bytes(djvu.encode_document(pages, quality="lossless"))
    for number, src in enumerate(pages, start=1):
        out = tmp_path / f"{number}.ppm"
        subprocess.run(
            ["ddjvu", "-format=ppm", f"-page={number}", str(path), str(out)], check=True
        )
        assert np.array_equal(np.asarray(Image.open(out).convert("RGB")), src[..., :3])


def written_page(number: int) -> np.ndarray:
    """A black-and-white page of text: the letters repeat from page to page."""
    image = Image.new("1", (400, 300), 1)
    draw = ImageDraw.Draw(image)
    for line in range(12):
        text = f"The quick brown fox jumps over lazy dog {number * 100 + line}"
        draw.text((10, 10 + 22 * line), text, fill=0)
    return np.asarray(image.convert("RGBA"))


def test_document_shares_shapes_between_pages():
    pages = [written_page(n) for n in range(4)]
    shared = djvu.encode_document(pages, quality="lossless")
    apart = djvu.encode_document(pages, quality="lossless", shared_dict_pages=99)
    merged = djvu.merge([djvu.encode_page(page, quality="lossless") for page in pages])
    assert len(shared) < len(apart)
    assert len(shared) < len(merged)
    for data in (shared, apart):
        for rendered, src in zip(render_all(data), pages):
            assert np.array_equal(rendered, src)


def test_document_auto_picks_one_profile():
    bilevel = [text_page(1), text_page(2)]
    assert djvu.encode_document(bilevel) == djvu.encode_document(bilevel, quality="lossless")
    mixed = [text_page(1), document_like_rgba()]
    assert djvu.encode_document(mixed) == djvu.encode_document(mixed, quality="quality")


@pytest.mark.parametrize("quality", ["quality", "archival"])
def test_document_layered_profiles(quality):
    pages = [document_like_rgba(), document_like_rgba()[:, ::-1].copy()]
    rendered = render_all(djvu.encode_document(pages, quality=quality))
    assert len(rendered) == 2
    for out, src in zip(rendered, pages):
        assert out.shape == src.shape
        diff = np.abs(out[..., :3].astype(int) - src[..., :3].astype(int))
        assert diff.mean() < 12


def test_document_accepts_every_page_kind_and_size():
    small = np.full((40, 50), 255, dtype=np.uint8)
    small[10:20, 10:30] = 0
    pages = [
        Image.fromarray(text_page(1)),
        small,
        djvu.Document.from_bytes(djvu.encode_page(text_page(2))).page(0).render(),
    ]
    doc = djvu.Document.from_bytes(djvu.encode_document(pages, quality="lossless"))
    assert [(doc.page(i).width, doc.page(i).height) for i in range(3)] == [
        (WIDTH, HEIGHT),
        (50, 40),
        (WIDTH, HEIGHT),
    ]


class LazyPages:
    """A sequence that builds each page on access and counts the reads."""

    def __init__(self, count: int, make=text_page):
        self.count, self.make, self.reads = count, make, []

    def __len__(self) -> int:
        return self.count

    def __getitem__(self, index: int) -> np.ndarray:
        self.reads.append(index)
        return self.make(index + 1)


def test_document_reads_a_lazy_sequence_by_index():
    eager = djvu.encode_document([text_page(n) for n in (1, 2, 3)], quality="lossless")
    lazy = LazyPages(3)
    assert djvu.encode_document(lazy, quality="lossless") == eager
    assert lazy.reads == [0, 1, 2]

    layered = LazyPages(3, make=lambda n: document_like_rgba())
    assert djvu.Document.from_bytes(djvu.encode_document(layered, quality="quality")).page_count() == 3
    assert layered.reads == [0, 1, 2]

    auto = LazyPages(2)
    djvu.encode_document(auto)
    assert auto.reads == [0, 1, 0, 1]


def test_document_accepts_a_generator():
    eager = djvu.encode_document([text_page(1), text_page(2)], quality="lossless")
    assert djvu.encode_document((text_page(n) for n in (1, 2)), quality="lossless") == eager


def test_document_single_page():
    data = djvu.encode_document([text_page(1)], quality="lossless")
    assert np.array_equal(render_all(data)[0], text_page(1))


def test_document_thumbnails():
    pages = [text_page(1), text_page(2)]
    plain = djvu.encode_document(pages, quality="lossless")
    with_thumbs = djvu.encode_document(pages, quality="lossless", thumbnails=True)
    assert len(with_thumbs) > len(plain)
    assert np.array_equal(render_all(with_thumbs)[1], pages[1])


def test_document_result_can_be_edited_and_merged():
    book = djvu.encode_document([text_page(1), text_page(2)], quality="lossless")
    joined = djvu.merge([book, djvu.encode_page(text_page(3))])
    assert djvu.Document.from_bytes(joined).page_count() == 3
    assert djvu.Document.from_bytes(djvu.remove_pages(book, [0])).page_count() == 1


@pytest.mark.parametrize("quality", ["lossless", "quality"])
def test_document_page_error_passes_through(quality):
    class Broken(LazyPages):
        def __getitem__(self, index):
            if index == 1:
                raise KeyError("no scan 2")
            return super().__getitem__(index)

    with pytest.raises(KeyError, match="no scan 2"):
        djvu.encode_document(Broken(3), quality=quality)
    with pytest.raises(TypeError, match="image must be"):
        djvu.encode_document([text_page(1), "page.png"], quality=quality)


def test_document_rejects_bad_arguments():
    pages = [text_page(1)]
    with pytest.raises(ValueError, match="pages is empty"):
        djvu.encode_document([])
    with pytest.raises(ValueError, match="quality must be"):
        djvu.encode_document(pages, quality="photo")
    with pytest.raises(ValueError, match="dpi"):
        djvu.encode_document(pages, dpi=0)
    with pytest.raises(ValueError, match="shared_dict_pages"):
        djvu.encode_document(pages, shared_dict_pages=0)
    with pytest.raises(TypeError, match="sequence or an iterable"):
        djvu.encode_document(42)
