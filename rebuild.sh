#!/bin/sh
# Rebuilds the WASM core and keeps both index.html (self-contained, with the
# binary inlined as base64) and dev.html/wasm-pkg (separate-file dev build)
# in sync. Run this after ANY change to wasm-core/src/lib.rs -- previously
# wasm-pkg got rebuilt but index.html's embedded copy did not, silently
# leaving the deployed page running a stale binary for several commits.
set -e
cd "$(dirname "$0")"

cd wasm-core
cargo build --release --target wasm32-unknown-unknown
cd ..
wasm-bindgen wasm-core/target/wasm32-unknown-unknown/release/wasm_core.wasm --target web --out-dir wasm-pkg

python3 - <<'PYEOF'
import re, base64

with open('wasm-pkg/wasm_core_bg.wasm', 'rb') as f:
    b64 = base64.b64encode(f.read()).decode('ascii')

with open('index.html') as f:
    html = f.read()
m = re.search(r'const WASM_B64 = "[^"]+";', html)
assert m, "WASM_B64 constant not found in index.html"
html = html[:m.start()] + f'const WASM_B64 = "{b64}";' + html[m.end():]
with open('index.html', 'w') as f:
    f.write(html)

with open('index.html') as f:
    html = f.read()
m = re.search(r'<script>\nconst WASM_B64.*?\n\};\n', html, re.S)
assert m, "inline wasm block not found in index.html"
new_prelude = "<script type=\"module\">\nimport init, * as wasmMod from './wasm-pkg/wasm_core.js';\n"
html2 = html[:m.start()] + new_prelude + html[m.end():]
old_boot = re.search(r"\(async function boot\(\)\{\n.*?\}\)\(\);", html2, re.S)
assert old_boot, "boot block not found in index.html"
new_boot = '''(async function boot(){
  wasmExports = await init();
  initGL();
  resizeCanvas();
  ready = true;
  if(location.hash.startsWith('#c=')){
    try{ loadConfig(decodeConfig(base64UrlToBytes(location.hash.slice(3)))); }
    catch(e){ console.warn('failed to load shared config from URL', e); }
  }
  generate();
})();'''
html3 = html2[:old_boot.start()] + new_boot + html2[old_boot.end():]
with open('dev.html', 'w') as f:
    f.write(html3)

print("index.html and dev.html both updated from wasm-pkg/wasm_core_bg.wasm")
PYEOF
