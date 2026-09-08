# FractalChess

Not a true fractal, but it produces surprisingly rich, self-similar-looking patterns from a very simple rule — inspired by a Numberphile video.

## The rule

Cells are numbered outward from the center in a square spiral (0, 1, 2, 3, ...). Pick a set of colors, each assigned one chess-like "leaper" piece (knight, wazir, ferz, dabbaba, alfil, dromedary, zebra, antelope, or a custom jump). Colors take turns in order. On its turn, a color places its piece on the lowest-numbered empty cell that isn't currently under attack by another color's pieces already on the board.

That's it — no other rules. Run it for a while and watch the boundaries between colors' territory.

## Try it

Open `index.html` directly in a browser — it's fully self-contained (the simulation core is a WebAssembly module embedded inline), no server or build step needed. Pick some pieces and colors, hit Generate, then drag to pan and scroll to zoom around the result.

## How it's built

- `wasm-core/` — the simulation core (spiral indexing, turn-based placement, attack checks) written in Rust, compiled to WebAssembly.
- `index.html` — the self-contained build: UI, WebGL renderer, and the compiled WASM module inlined as base64. This is what GitHub Pages serves.
- `dev.html` + `wasm-pkg/` — a dev-mode version that loads the WASM module as a separate file (via `wasm-bindgen`'s generated JS glue) instead of inlining it, so you can rebuild the Rust side and reload without re-encoding. Needs to be served over http(s) (e.g. `python3 -m http.server`), not opened via `file://`.

### Rebuilding the WASM core

```
cd wasm-core
cargo build --release --target wasm32-unknown-unknown
cd ..
wasm-bindgen wasm-core/target/wasm32-unknown-unknown/release/wasm_core.wasm --target web --out-dir wasm-pkg
```

That updates `wasm-pkg/` for `dev.html`. To refresh the inlined copy in `index.html`, base64-encode `wasm-pkg/wasm_core_bg.wasm` and swap it into the `WASM_B64` constant near the top of `index.html`'s script.

## Rendering notes

The simulation resolves cells out to the edge of the current view size (plus a small halo, sized to the largest piece's jump, so edge cells still see every square that could attack them) once per Generate click. Panning and zooming afterward are pure camera moves over that finished image — no re-simulation, no merging cells into pixels.

The most interesting structure tends to sit close to the origin; further out the patterns settle into repeating territory, so there's little point generating detail far larger than what the view can actually show.
