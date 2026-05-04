# djvu-rs WASM demo

Browser-based DjVu viewer powered by djvu-rs compiled to WebAssembly.

## Build

```sh
# Install wasm-pack if you haven't already:
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# From the repo root — builds pkg/ inside this directory:
wasm-pack build --target web --out-dir examples/wasm/pkg

# Serve locally (any static file server works):
python3 -m http.server 8080 --directory examples/wasm
# Then open http://localhost:8080
```

## Lazy HTTP Range loading

`range_lazy.md` shows the `wasm32` integration shape for large remote books:
implement an `AsyncRead + AsyncSeek` reader that fetches `Range:
bytes=start-end` with `gloo::net::http::Request`, then pass it to
`djvu_rs::djvu_async::from_async_reader_lazy_local`.

## npm package

```sh
# Publish to npm (requires npm login):
wasm-pack build --target web --out-dir examples/wasm/pkg --release
cd examples/wasm/pkg
npm publish
```
