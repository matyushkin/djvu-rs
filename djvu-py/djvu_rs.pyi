"""Type hints for djvu_rs, the Python bindings of the djvu-rs library.

Annotations, text layers, metadata and bookmarks cross the boundary as plain
dicts and lists. The readers return the full shapes below (`Metadata`,
`Annotation`, ...). The editor also accepts the `...Input` shapes, where keys
left out take their defaults.
"""

from __future__ import annotations

from typing import Any, List, Literal, Optional, Sequence, Tuple, TypedDict, Union, final

__all__ = [
    "__version__",
    "Error",
    "DecodeError",
    "IoError",
    "ExportError",
    "PageIndexError",
    "EditError",
    "Document",
    "Page",
    "Editor",
    "Pixmap",
    "Annotation",
    "AnnotationInput",
    "Bookmark",
    "BookmarkInput",
    "Border",
    "Color",
    "Highlight",
    "LineShape",
    "MapArea",
    "MapAreaInput",
    "Metadata",
    "MetadataInput",
    "OvalShape",
    "PolyShape",
    "Rect",
    "RectShape",
    "Shape",
    "TextLayer",
    "TextLayerInput",
    "TextShape",
    "TextZone",
    "TextZoneInput",
    "ZoneKind",
]

__version__: str

# ---- Exceptions -----------------------------------------------------------------

class Error(Exception):
    """Base exception for all djvu-rs Python binding errors."""

class DecodeError(Error):
    """Document parse, decode, or render failure."""

class IoError(Error):
    """Filesystem or I/O failure while opening a DjVu document."""

class ExportError(Error):
    """Conversion to PDF, EPUB, CBZ or TIFF failed."""

class PageIndexError(IndexError):
    """Page index is out of range for this document."""

class EditError(Error):
    """An edit was rejected, or the edited document could not be saved."""

# ---- Document model as dicts ----------------------------------------------------

class Metadata(TypedDict):
    """Document metadata. `extra` holds other `(key, value)` pairs in order."""

    title: Optional[str]
    author: Optional[str]
    subject: Optional[str]
    publisher: Optional[str]
    year: Optional[str]
    keywords: Optional[str]
    extra: List[Tuple[str, str]]

class MetadataInput(TypedDict, total=False):
    """Metadata to write; keys left out are empty."""

    title: Optional[str]
    author: Optional[str]
    subject: Optional[str]
    publisher: Optional[str]
    year: Optional[str]
    keywords: Optional[str]
    extra: Sequence[Tuple[str, str]]

class Bookmark(TypedDict):
    """One table-of-contents entry; `children` has the same shape."""

    title: str
    url: str
    children: List[Bookmark]

class _BookmarkRequired(TypedDict):
    title: str
    url: str

class BookmarkInput(_BookmarkRequired, total=False):
    """A bookmark to write; `children` defaults to none."""

    children: Sequence[Union[Bookmark, BookmarkInput]]

class Color(TypedDict):
    r: int
    g: int
    b: int

class Rect(TypedDict):
    """A rectangle: DjVu coordinates (bottom-left origin) in annotations,
    top-left origin in text layers."""

    x: int
    y: int
    width: int
    height: int

@final
class RectShape(TypedDict):
    Rect: Rect

@final
class OvalShape(TypedDict):
    Oval: Rect

@final
class PolyShape(TypedDict):
    Poly: Sequence[Tuple[int, int]]

@final
class LineShape(TypedDict):
    Line: Tuple[int, int, int, int]

@final
class TextShape(TypedDict):
    Text: Rect

Shape = Union[RectShape, OvalShape, PolyShape, LineShape, TextShape]
"""A map-area shape: a dict with exactly one key."""

class Border(TypedDict):
    """A DjVuLibre border option without parentheses, such as `"xor"`,
    `"border #FF0000"` or `"shadow_in 3"`."""

    style: str

class Highlight(TypedDict):
    color: Color

class Annotation(TypedDict):
    """Page settings. `extra` keeps every other top-level form, such as
    `(metadata ...)` or `(align center top)`, as S-expression text."""

    background: Optional[Color]
    zoom: Optional[int]
    mode: Optional[str]
    extra: List[str]

class AnnotationInput(TypedDict, total=False):
    """Page settings to write; keys left out take their defaults."""

    background: Optional[Color]
    zoom: Optional[int]
    mode: Optional[str]
    extra: Sequence[str]

class MapArea(TypedDict):
    """A link or highlighted region of a page."""

    url: str
    target: Optional[str]
    description: str
    shape: Shape
    border: Optional[Border]
    highlight: Optional[Highlight]
    extra: List[str]

class _MapAreaRequired(TypedDict):
    shape: Shape

class MapAreaInput(_MapAreaRequired, total=False):
    """A map area to write; only `shape` is required."""

    url: str
    target: Optional[str]
    description: str
    border: Optional[Border]
    highlight: Optional[Highlight]
    extra: Sequence[str]

ZoneKind = Literal["Page", "Column", "Region", "Para", "Line", "Word", "Character"]

class TextZone(TypedDict):
    """A zone of the text layer; `text` is a span of the layer's text."""

    kind: ZoneKind
    rect: Rect
    text: str
    children: List[TextZone]

class _TextZoneRequired(TypedDict):
    kind: ZoneKind
    rect: Rect
    text: str

class TextZoneInput(_TextZoneRequired, total=False):
    children: Sequence[Union[TextZone, TextZoneInput]]

class TextLayer(TypedDict):
    """The text of a page plus its zone tree."""

    text: str
    zones: List[TextZone]

class TextLayerInput(TypedDict):
    text: str
    zones: Sequence[Union[TextZone, TextZoneInput]]

# ---- Classes --------------------------------------------------------------------

@final
class Pixmap:
    """An RGBA pixel buffer. Supports the buffer protocol (`memoryview`)."""

    @property
    def width(self) -> int: ...
    @property
    def height(self) -> int: ...
    def data(self) -> bytes:
        """RGBA pixel data (length = width * height * 4)."""
    def to_numpy(self) -> Any:
        """A numpy array of shape (height, width, 4), dtype uint8."""
    def to_pil(self) -> Any:
        """A PIL.Image.Image in RGBA mode."""
    def to_numpy_zerocopy(self) -> Any:
        """Like `to_numpy`, backed by this buffer; treat it as read-only."""
    def to_pil_zerocopy(self) -> Any:
        """Like `to_pil`, backed by this buffer; treat it as read-only."""
    def __buffer__(self, flags: int, /) -> memoryview: ...

@final
class Page:
    """A page within a DjVu document."""

    @property
    def width(self) -> int:
        """Page width in pixels."""
    @property
    def height(self) -> int:
        """Page height in pixels."""
    @property
    def dpi(self) -> int:
        """Page DPI."""
    @property
    def bg44_chunk_count(self) -> int:
        """Number of BG44 refinement chunks (0 for bilevel pages)."""
    def render(self, dpi: Optional[float] = None) -> Pixmap:
        """Render the page; native DPI when `dpi` is None."""
    def render_region(
        self,
        x: int,
        y: int,
        w: int,
        h: int,
        full_width: Optional[int] = None,
        full_height: Optional[int] = None,
    ) -> Pixmap:
        """Render a rectangle cut from a render of size full_width x full_height."""
    def render_coarse(self, dpi: Optional[float] = None) -> Optional[Pixmap]:
        """A fast, blurry preview; None for bilevel-only pages."""
    def render_progressive(self, chunk_n: int, dpi: Optional[float] = None) -> Pixmap:
        """Render with BG44 chunks 0..=chunk_n."""
    def text(self) -> Optional[str]:
        """The page text, or None when there is no text layer."""
    def annotations(self) -> Optional[Tuple[Annotation, List[MapArea]]]:
        """The page's annotations, or None when it has none."""
    def text_layer(self) -> Optional[TextLayer]:
        """The page's text layer, or None when it has none."""

@final
class Document:
    """A DjVu document opened for reading."""

    @staticmethod
    def open(path: str) -> Document: ...
    @staticmethod
    def from_bytes(data: bytes) -> Document: ...
    def page_count(self) -> int: ...
    def page(self, index: int) -> Page:
        """The page at 0-based `index`; raises PageIndexError."""
    def metadata(self) -> Optional[Metadata]: ...
    def bookmarks(self) -> List[Bookmark]: ...
    def to_pdf(
        self,
        dpi: int = 150,
        jpeg_quality: Optional[int] = 80,
        adaptive: bool = False,
        ccitt_g4: bool = False,
        mrc: bool = False,
    ) -> bytes: ...
    def write_pdf(
        self,
        path: str,
        dpi: int = 150,
        jpeg_quality: Optional[int] = 80,
        adaptive: bool = False,
        ccitt_g4: bool = False,
        mrc: bool = False,
    ) -> None: ...
    def to_epub(
        self,
        dpi: int = 150,
        title: str = "DjVu Document",
        author: str = "",
        language: str = "en",
        modified: Optional[str] = None,
        reflowable_text: bool = False,
        jpeg_quality: Optional[int] = None,
        adaptive: bool = False,
    ) -> bytes: ...
    def write_epub(
        self,
        path: str,
        dpi: int = 150,
        title: str = "DjVu Document",
        author: str = "",
        language: str = "en",
        modified: Optional[str] = None,
        reflowable_text: bool = False,
        jpeg_quality: Optional[int] = None,
        adaptive: bool = False,
    ) -> None: ...
    def to_cbz(
        self, dpi: int = 150, rotation: int = 0, pages: Optional[Sequence[int]] = None
    ) -> bytes: ...
    def write_cbz(
        self,
        path: str,
        dpi: int = 150,
        rotation: int = 0,
        pages: Optional[Sequence[int]] = None,
    ) -> None: ...
    def to_tiff(
        self,
        mode: Literal["color", "bilevel"] = "color",
        scale: float = 1.0,
        bilevel_compression: Literal["deflate", "g4"] = "deflate",
    ) -> bytes: ...
    def write_tiff(
        self,
        path: str,
        mode: Literal["color", "bilevel"] = "color",
        scale: float = 1.0,
        bilevel_compression: Literal["deflate", "g4"] = "deflate",
    ) -> None: ...

@final
class Editor:
    """A DjVu document opened for editing. Changes stay in memory until
    `save` or `to_bytes`."""

    @staticmethod
    def open(path: str) -> Editor: ...
    @staticmethod
    def from_bytes(data: bytes) -> Editor: ...
    @property
    def modified(self) -> bool:
        """True once any change has been made."""
    def page_count(self) -> int: ...
    def document(self) -> Document:
        """The document with the changes so far, read-only."""
    def metadata(self) -> Optional[Metadata]: ...
    def set_metadata(self, metadata: Union[Metadata, MetadataInput]) -> None: ...
    def remove_metadata(self) -> None: ...
    def bookmarks(self) -> List[Bookmark]: ...
    def set_bookmarks(self, bookmarks: Sequence[Union[Bookmark, BookmarkInput]]) -> None: ...
    def page_annotations(self, index: int) -> Optional[Tuple[Annotation, List[MapArea]]]: ...
    def set_page_annotations(
        self,
        index: int,
        annotation: Union[Annotation, AnnotationInput],
        areas: Optional[Sequence[Union[MapArea, MapAreaInput]]] = None,
    ) -> None: ...
    def remove_page_annotations(self, index: int) -> None: ...
    def page_text_layer(self, index: int) -> Optional[TextLayer]: ...
    def set_page_text_layer(self, index: int, layer: Union[TextLayer, TextLayerInput]) -> None: ...
    def remove_page_text_layer(self, index: int) -> None: ...
    def to_bytes(self) -> bytes: ...
    def save(self, path: str) -> None: ...
