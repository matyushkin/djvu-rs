# djvu-rs

Pure-Rust tooling for the DjVu document format: reading, rendering, converting,
and creating DjVu files, delivered through several consumption channels.

## Language

### Documentation & audience

**Reader**:
The primary audience of the README — a person or an LLM agent who arrives with
a DjVu task (convert, extract text, render, create) and must decide within
seconds whether this project covers it.
_Avoid_: user, developer (too narrow — the Reader may not write Rust at all)

**Task**:
A user-level goal expressed in outcome terms (e.g. "DjVu → PDF", "extract
text", "show DjVu in a browser"), as opposed to a codec capability (e.g. "IW44
wavelet decoder") which is an implementation detail behind it.
_Avoid_: feature (ambiguous — collides with Cargo feature flags)

**Channel**:
A way to consume the project: Rust crate, CLI binary, WebAssembly bindings, or
Python bindings. All channels ship from this one repository and share the crate
version.
_Avoid_: binding, target, platform
