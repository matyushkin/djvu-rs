# Local dev + CI runner for the djvu-py bindings crate.
#
# Builds the extension module via maturin into a uv-managed venv and runs the
# pytest suite (tests/). Mirrors what the `djvu-py` CI job does.
.PHONY: py-venv py-build py-test py-clean

VENV := .venv
PYTHON_VERSION ?= 3.12

## Create the uv-managed venv and install maturin + test deps (numpy, Pillow, pytest).
py-venv:
	uv venv $(VENV) --python $(PYTHON_VERSION)
	uv pip install --python $(VENV)/bin/python maturin pytest numpy pillow

## Build djvu-py in release mode and install it into the venv (editable).
py-build: py-venv
	VIRTUAL_ENV=$(abspath $(VENV)) PATH="$(abspath $(VENV))/bin:$$PATH" \
		$(VENV)/bin/maturin develop --release

## Build (if needed) and run the pytest suite.
py-test: py-build
	$(VENV)/bin/pytest tests -v

## Remove the venv and pytest caches.
py-clean:
	rm -rf $(VENV) .pytest_cache tests/__pycache__
