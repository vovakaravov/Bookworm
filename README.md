<p align="center">
  <img src="assets/bookworm.svg" alt="Bookworm" width="88" height="88" />
</p>

<h1 align="center">Bookworm</h1>

<p align="center">
  Native, GPU-accelerated Rust + <a href="https://gpui.rs">GPUI</a> digital book library and distraction-free EPUB reader for Linux desktop and Android — styled in <strong>Kanagawa Dragon</strong>.
</p>

---

## Structure

- `core/` (`bookworm-core`) — Shared EPUB parser/writer, library catalog, metadata lookup (`online-metadata` feature), Markdown highlight/note export, and Syncthing conflict merger.
- `desktop/` (`bookworm`) — Linux desktop library & EPUB reader.
- `mobile/` (`bookworm-mobile`) — Android library & EPUB reader powered by `gpui-mobile`.
- `assets/bookworm.svg` — Application icon.

## Build & Deploy

```bash
# Linux desktop
cd desktop && cargo build --release

# Android (requires cargo-ndk, Android SDK/NDK, and adb)
cd mobile && ./deploy.sh
```

## License

MIT — see [LICENSE](LICENSE).
