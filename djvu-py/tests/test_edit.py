"""Reading and editing annotations, text layers, metadata and bookmarks."""

from __future__ import annotations

import pytest

import djvu_rs as djvu


LINK = {
    "url": "https://example.org",
    "description": "example",
    "shape": {"Rect": {"x": 10, "y": 20, "width": 100, "height": 30}},
}


def reopen(editor):
    return djvu.Document.from_bytes(editor.to_bytes())


# ---- Readers -------------------------------------------------------------------


def test_bookmarks_are_nested_dicts(navm_path):
    marks = djvu.Document.open(str(navm_path)).bookmarks()
    assert marks
    assert set(marks[0]) == {"title", "url", "children"}
    assert marks[0]["title"] == "Links"


def test_bookmarks_empty_when_absent(boy_path):
    assert djvu.Document.open(str(boy_path)).bookmarks() == []


def test_annotations_are_dicts(navm_path):
    annotation, areas = djvu.Document.open(str(navm_path)).page(0).annotations()
    assert "extra" in annotation
    assert areas
    assert areas[0]["url"] == "#1"
    assert "Rect" in areas[0]["shape"]


def test_annotations_none_when_absent(boy_path):
    assert djvu.Document.open(str(boy_path)).page(0).annotations() is None


def test_text_layer_is_a_zone_tree(multipage_path):
    layer = djvu.Document.open(str(multipage_path)).page(0).text_layer()
    assert layer["text"].strip()
    root = layer["zones"][0]
    assert root["kind"] == "Page"
    assert set(root["rect"]) == {"x", "y", "width", "height"}


def test_metadata_none_when_absent(boy_path):
    assert djvu.Document.open(str(boy_path)).metadata() is None


# ---- Editor --------------------------------------------------------------------


def test_unedited_bytes_are_unchanged(boy_path, boy_bytes):
    editor = djvu.Editor.open(str(boy_path))
    assert not editor.modified
    assert editor.to_bytes() == boy_bytes


def test_metadata_accepts_a_partial_dict(boy_path):
    editor = djvu.Editor.open(str(boy_path))
    editor.set_metadata({"title": "Atlas", "extra": [("isbn", "123")]})
    assert editor.modified
    meta = reopen(editor).metadata()
    assert meta["title"] == "Atlas"
    assert meta["author"] is None
    assert [tuple(pair) for pair in meta["extra"]] == [("isbn", "123")]

    editor.remove_metadata()
    assert reopen(editor).metadata() is None


def test_readers_show_unsaved_changes(boy_path):
    editor = djvu.Editor.open(str(boy_path))
    assert editor.metadata() is None
    editor.set_metadata({"title": "Draft"})
    assert editor.metadata()["title"] == "Draft"
    assert editor.document().metadata()["title"] == "Draft"


def test_annotations_round_trip(navm_path):
    editor = djvu.Editor.open(str(navm_path))
    annotation, areas = editor.page_annotations(0)
    editor.set_page_annotations(0, annotation, [*areas, LINK])

    _, written = reopen(editor).page(0).annotations()
    assert written[:-1] == areas
    assert written[-1]["url"] == "https://example.org"
    assert written[-1]["description"] == "example"
    assert written[-1]["shape"] == LINK["shape"]


def test_annotations_on_a_page_without_them(boy_path):
    editor = djvu.Editor.open(str(boy_path))
    editor.set_page_annotations(0, {"zoom": 150}, [LINK])
    annotation, areas = reopen(editor).page(0).annotations()
    assert annotation["zoom"] == 150
    assert len(areas) == 1

    editor.remove_page_annotations(0)
    assert reopen(editor).page(0).annotations() is None


def test_bookmarks_round_trip(navm_path):
    editor = djvu.Editor.open(str(navm_path))
    marks = [{"title": "Start", "url": "#1", "children": [{"title": "Two", "url": "#2"}]}]
    editor.set_bookmarks(marks)
    read = reopen(editor).bookmarks()
    assert read[0]["title"] == "Start"
    assert read[0]["children"][0] == {"title": "Two", "url": "#2", "children": []}

    editor.set_bookmarks([])
    assert reopen(editor).bookmarks() == []


def test_bookmarks_need_a_bundle(boy_path):
    editor = djvu.Editor.open(str(boy_path))
    with pytest.raises(djvu.EditError):
        editor.set_bookmarks([{"title": "x", "url": "#1"}])


def test_text_layer_round_trip(multipage_path):
    editor = djvu.Editor.open(str(multipage_path))
    layer = editor.page_text_layer(0)
    editor.set_page_text_layer(0, layer)
    assert reopen(editor).page(0).text_layer() == layer

    editor.remove_page_text_layer(0)
    doc = reopen(editor)
    assert doc.page(0).text_layer() is None
    assert doc.page(1).text_layer() is not None


def test_new_text_layer(boy_path):
    editor = djvu.Editor.open(str(boy_path))
    word = {"kind": "Word", "rect": {"x": 5, "y": 6, "width": 40, "height": 12}, "text": "boy"}
    page = {
        "kind": "Page",
        "rect": {"x": 0, "y": 0, "width": 192, "height": 256},
        "text": "boy",
        "children": [word],
    }
    editor.set_page_text_layer(0, {"text": "boy", "zones": [page]})
    layer = reopen(editor).page(0).text_layer()
    assert layer["text"] == "boy"
    assert layer["zones"][0]["children"][0]["rect"] == word["rect"]


def test_save_replaces_the_opened_file(tmp_path, boy_bytes):
    path = tmp_path / "boy.djvu"
    path.write_bytes(boy_bytes)
    editor = djvu.Editor.open(str(path))
    editor.set_metadata({"title": "Saved"})
    editor.save(str(path))
    assert djvu.Document.open(str(path)).metadata()["title"] == "Saved"
    assert [p.name for p in tmp_path.iterdir()] == ["boy.djvu"]


def test_invalid_dict_raises_value_error(boy_path):
    editor = djvu.Editor.open(str(boy_path))
    with pytest.raises(ValueError, match="map areas"):
        editor.set_page_annotations(0, {}, [{"url": "no shape"}])
    assert not editor.modified


def test_page_out_of_range(boy_path):
    editor = djvu.Editor.open(str(boy_path))
    with pytest.raises(djvu.PageIndexError):
        editor.remove_page_annotations(5)
    with pytest.raises(IndexError):
        editor.page_annotations(5)


def test_legacy_page_is_read_only(legacy_path):
    editor = djvu.Editor.open(str(legacy_path))
    with pytest.raises(djvu.EditError, match="legacy"):
        editor.set_page_annotations(0, {})
