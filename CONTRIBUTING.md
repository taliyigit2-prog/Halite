# Contributing to Halite

Thanks for your interest in contributing! Halite is a free, open-source AI
music toolkit. Contributions of all kinds are welcome.

## How to contribute

1. **Fork** the repository and clone it locally.
2. Create a new branch for your change: `git checkout -b my-feature`.
3. Make your changes, keeping them focused and readable.
4. Run `cargo check` in `src-tauri/` to verify the Rust code compiles.
5. Commit with a clear message and open a pull request.

## Development setup

Requirements:

- [Rust](https://rustup.rs/) (1.92 or newer)
- [Node.js](https://nodejs.org/) (18 or newer)
- Platform build dependencies for [Tauri v2](https://tauri.app/start/prerequisites/)

Then:

```bash
npm install
npm run tauri dev      # run in development
npm run tauri build    # produce a release bundle
```

## Project layout

- `src/` — the web frontend (vanilla HTML/CSS/JS, no bundler)
- `src/locales/` — translations (JSON)
- `src-tauri/` — the Rust backend (Tauri + ONNX Runtime)

## Code of conduct

Be kind, be constructive. Respect everyone's time and contributions.

## License

By contributing, you agree that your contributions will be licensed under
the MIT License.
