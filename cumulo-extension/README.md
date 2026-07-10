# cumulo-extension

The `cumulo-web` app packaged as an unpacked Chrome extension. Clicking the
toolbar icon opens a popup that adds the current page to cumulo (like a
bookmark) and links to open cumulo in a full-page tab.

## Build

Run from this directory; the bundle lands in `dist/`.

```sh
cd cumulo-extension
trunk build            # development
trunk build --release  # distribution (wasm-opt enabled)
```

## Load into Chrome

1. Open `chrome://extensions`.
2. Turn on **Developer mode** (top right).
3. **Load unpacked** and select `cumulo-extension/dist`.
4. Click the extension's toolbar icon; the popup adds the current page or opens cumulo.

Rebuild after changing code, then press **Reload** on the extension.
