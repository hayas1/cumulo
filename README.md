# cumulo

A tool for reaching half-remembered resources fast, by association from just a few clues.

[![Main](https://github.com/hayas1/cumulo/actions/workflows/main.yml/badge.svg)](https://github.com/hayas1/cumulo/actions/workflows/main.yml)

**▶ [Try the live demo](https://hayas1.github.io/cumulo/)** — runs entirely in your browser; nothing is uploaded.

<!-- screenshot: drop an image at docs/screenshot.png and reference it here -->

## Why

Bookmarks store links; cumulo helps you *recall* one. You reach a resource — a cloud console, a team portal — by association from a few clues (*which platform* × *which environment* × *which team*), not by remembering where you filed it.

Those aspects are orthogonal, so cumulo does not force them into one folder tree. It keeps **a separate tree per aspect (a facet)** and classifies each resource with one value from each. Filtering then ANDs a few facets together to converge on one resource quickly.

The upshot: the payload is not the stored data but the **classification** — and organizing it is worth doing in itself (a shared classification doubles as onboarding material). See [`docs/domain.md`](docs/domain.md) for the full model.

## Try it

- **Web** — open the [live demo](https://hayas1.github.io/cumulo/). Two views:
  - **Facet view** — filter by facet values, or type a clue into the palette, to narrow the list.
  - **Map view** — a force-laid map of resources you can zoom and drill into by axis.
  - State lives in the URL (deep-linkable) and in browser storage (survives reloads).
- **Chrome extension** — grab the unpacked bundle from [Releases](https://github.com/hayas1/cumulo/releases), or build it yourself. The toolbar icon adds the current page to cumulo and opens it in a full-page tab. See [`cumulo-extension/README.md`](cumulo-extension/README.md).

## Workspace

A Rust workspace; the app is [Leptos](https://leptos.dev/) compiled to WebAssembly (client-side only, no backend).

| Crate | What it is |
|---|---|
| [`cumulo-model`](cumulo-model) | The domain model — the bipartite graph of resources and categories, and its (de)serialization. |
| [`cumulo-web`](cumulo-web) | The Leptos/WASM app: facet and map views, forms, filtering, storage. |
| [`cumulo-extension`](cumulo-extension/README.md) | `cumulo-web` packaged as an unpacked Chrome (MV3) extension. |
| [`cumulo-e2e`](cumulo-e2e/README.md) | Browser-driven end-to-end scenarios (headless Chromium via CDP), gated behind the `browser` feature. |

## Development

Prerequisites: a Rust toolchain and [Trunk](https://trunkrs.dev/) (`cargo install trunk`).

```sh
# Run the web app locally
cd cumulo-web && trunk serve

# Run unit/integration tests across the workspace (no browser needed)
cargo test --all

# Run the end-to-end scenarios (needs a built app + Chromium)
( cd cumulo-web && trunk build )
cargo test -p cumulo-e2e --features browser -- --test-threads=1
```

Push to `main` builds `cumulo-web` and deploys it to GitHub Pages; a version bump also publishes the packaged extension to Releases.

## Docs

- [`docs/README.md`](docs/README.md) — what cumulo is and the scope of these docs.
- [`docs/domain.md`](docs/domain.md) — the domain model and the vocabulary (Resource/Catalog, Category/Taxonomy, axis, value, filtering).

## License

See [LICENSE](LICENSE).
