# cumulo-e2e

End-to-end scenarios that drive the built `cumulo-web` app in a real headless
Chromium via [chromiumoxide](https://crates.io/crates/chromiumoxide) (Chrome
DevTools Protocol). These cover screen behaviour that native unit tests cannot:
URL/state sync, history, drag-and-drop, map interaction, resource CRUD, and
persistence across reloads.

The whole crate is gated behind the `browser` feature, so `cargo test --all`
(which CI runs without a browser) compiles nothing here. Opt in with
`--features browser`.

## Prerequisites

- A Chromium binary. Either set `CUMULO_E2E_CHROME=/path/to/chrome`, or install
  the one used here:

  ```sh
  npx playwright install --with-deps chromium
  ```

  The harness auto-discovers `~/.cache/ms-playwright/chromium-*/chrome-linux64/chrome`.

- A built app. The harness serves `cumulo-web/dist`:

  ```sh
  ( cd cumulo-web && trunk build )
  ```

## Run

Each scenario launches its own browser, so run serially on a memory-constrained
host:

```sh
( cd cumulo-web && trunk build )                          # produce cumulo-web/dist
cargo test -p cumulo-e2e --features browser -- --test-threads=1
```
