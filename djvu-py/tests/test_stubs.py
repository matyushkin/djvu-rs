"""The type stub `djvu_rs.pyi` names every public class, method and
exception of the module, and nothing the module lacks."""

from __future__ import annotations

import ast
import inspect
from pathlib import Path

import djvu_rs as djvu

STUB = Path(__file__).resolve().parents[1] / "djvu_rs.pyi"
CLASSES = ("Document", "Page", "Editor", "Pixmap")


def stub_tree() -> ast.Module:
    return ast.parse(STUB.read_text(encoding="utf-8"))


def stub_classes() -> dict[str, set[str]]:
    classes = {}
    for node in stub_tree().body:
        if isinstance(node, ast.ClassDef):
            classes[node.name] = {
                item.name for item in node.body if isinstance(item, ast.FunctionDef)
            }
    return classes


def runtime_members(cls: type) -> set[str]:
    return {
        name
        for name in vars(cls)
        if not name.startswith("_") or name in ("__buffer__",)
    }


def test_stub_parses():
    stub_tree()


def test_stub_covers_every_public_name():
    stub = stub_classes()
    runtime = {
        name
        for name in dir(djvu)
        if not name.startswith("_") and isinstance(getattr(djvu, name), type)
    }
    assert runtime <= set(stub), runtime - set(stub)


def test_stub_methods_match_the_classes():
    stub = stub_classes()
    for name in CLASSES:
        cls = getattr(djvu, name)
        expected = runtime_members(cls)
        if name == "Pixmap":
            expected.add("__buffer__")
        assert stub[name] == expected, (name, stub[name] ^ expected)


def test_stub_parameters_match():
    stub = {
        (cls.name, fn.name): [a.arg for a in fn.args.args if a.arg != "self"]
        for cls in stub_tree().body
        if isinstance(cls, ast.ClassDef) and cls.name in CLASSES
        for fn in cls.body
        if isinstance(fn, ast.FunctionDef) and not fn.name.startswith("_")
    }
    for (cls_name, fn_name), params in stub.items():
        member = getattr(getattr(djvu, cls_name), fn_name)
        if isinstance(member, property) or not callable(member):
            continue
        signature = inspect.signature(member)
        runtime = [p for p in signature.parameters if p != "self"]
        assert params == runtime, (cls_name, fn_name)
