# cumulo-extension

The `cumulo-web` app packaged as an unpacked Chrome extension. Clicking the
toolbar icon opens cumulo in a full-page tab.

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
4. Click the extension's toolbar icon to open cumulo in a new tab.

Rebuild after changing code, then press **Reload** on the extension.
