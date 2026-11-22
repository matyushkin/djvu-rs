"""Module-level structural edits: merge, split, remove_pages."""

from __future__ import annotations

import pytest

import djvu_rs as djvu


def pages(data: bytes) -> list[tuple[int, int]]:
    doc = djvu.Document.from_bytes(data)
    return [(doc.page(i).width, doc.page(i).height) for i in range(doc.page_count())]


def pixels(data: bytes, index: int) -> bytes:
    return bytes(djvu.Document.from_bytes(data).page(index).render())


@pytest.fixture(scope="module")
def navm(navm_path) -> bytes:
    return navm_path.read_bytes()


@pytest.fixture(scope="module")
def boy(boy_path) -> bytes:
    return boy_path.read_bytes()


def test_merge_keeps_every_page_in_order(navm, boy):
    merged = djvu.merge([navm, boy])
    assert isinstance(merged, bytes)
    assert pages(merged) == pages(navm) + pages(boy)
    assert pixels(merged, 6) == pixels(boy, 0)
    assert pixels(merged, 2) == pixels(navm, 2)


def test_merge_accepts_bytearray(boy):
    assert pages(djvu.merge([bytearray(boy), boy])) == pages(boy) * 2


def test_split_takes_a_half_open_range(navm):
    part = djvu.split(navm, 1, 3)
    assert pages(part) == pages(navm)[1:3]
    assert pixels(part, 0) == pixels(navm, 1)


def test_split_and_merge_round_trip(navm):
    count = len(pages(navm))
    again = djvu.merge([djvu.split(navm, 0, 2), djvu.split(navm, 2, count)])
    assert pages(again) == pages(navm)
    assert all(pixels(again, i) == pixels(navm, i) for i in range(count))


def test_insert_a_page_with_split_and_merge(navm, boy):
    count = len(pages(navm))
    combined = djvu.merge([djvu.split(navm, 0, 2), boy, djvu.split(navm, 2, count)])
    assert pages(combined) == pages(navm)[:2] + pages(boy) + pages(navm)[2:]


def test_remove_pages(navm):
    rebuilt = djvu.remove_pages(navm, [0, 5])
    assert pages(rebuilt) == pages(navm)[1:5]
    assert pixels(rebuilt, 0) == pixels(navm, 1)
    before = djvu.Document.from_bytes(navm).bookmarks()
    assert djvu.Document.from_bytes(rebuilt).bookmarks() == before


def test_inputs_are_left_unchanged(navm):
    copy = bytes(navm)
    djvu.split(navm, 0, 1)
    djvu.remove_pages(navm, [0])
    djvu.merge([navm, navm])
    assert navm == copy


def test_out_of_range_pages_raise_page_index_error(navm):
    with pytest.raises(djvu.PageIndexError):
        djvu.split(navm, 4, 9)
    with pytest.raises(djvu.PageIndexError):
        djvu.split(navm, 2, 2)
    with pytest.raises(djvu.PageIndexError):
        djvu.remove_pages(navm, [9])


def test_rejected_edits_raise_edit_error(navm, boy):
    with pytest.raises(djvu.EditError):
        djvu.merge([])
    with pytest.raises(djvu.EditError):
        djvu.remove_pages(navm, [1, 1])
    with pytest.raises(djvu.EditError):
        djvu.remove_pages(navm, list(range(len(pages(navm)))))
    with pytest.raises(djvu.EditError, match="bundled"):
        djvu.remove_pages(boy, [0])


def test_bad_input_raises_decode_error():
    with pytest.raises(djvu.DecodeError):
        djvu.merge([b"not a djvu file"])
    with pytest.raises(djvu.DecodeError):
        djvu.split(b"not a djvu file", 0, 1)
    with pytest.raises(djvu.DecodeError):
        djvu.remove_pages(b"not a djvu file", [0])


def test_wrong_types_raise_type_error(navm):
    with pytest.raises(TypeError):
        djvu.merge(["book.djvu"])
    with pytest.raises(TypeError):
        djvu.split("book.djvu", 0, 1)
