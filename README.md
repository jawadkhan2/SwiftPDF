# SwiftPDF

Fast, native desktop app for merging, splitting and signing PDFs. Built with [Tauri](https://tauri.app/) (Rust) + [SvelteKit](https://kit.svelte.dev/), powered by [PDFium](https://pdfium.googlesource.com/pdfium/).

## Download

Get the latest installer for your platform from the [**Releases**](https://github.com/jawadkhan2/SwiftPDF/releases/latest) page:

| Platform | File |
| --- | --- |
| Windows | `.msi` or `.exe` |
| macOS (Apple Silicon) | `.dmg` (aarch64) |
| macOS (Intel) | `.dmg` (x64) |

> Windows builds are not code-signed. Windows SmartScreen may warn on first launch; choose "More info" -> "Run anyway" if you trust the download.

### Opening SwiftPDF on macOS

SwiftPDF's macOS builds are ad-hoc signed, but they are not Apple Developer ID signed or notarized. macOS may still show a warning such as "SwiftPDF is damaged and can't be opened" or ask you to move the app to the Trash.

Only use these steps for a SwiftPDF `.dmg` downloaded from the official [Releases](https://github.com/jawadkhan2/SwiftPDF/releases/latest) page:

1. Drag `SwiftPDF.app` from the `.dmg` into your `Applications` folder.
2. Try opening SwiftPDF from `Applications`.
3. If macOS blocks it, open **System Settings** -> **Privacy & Security**, scroll to the **Security** section, and choose **Open Anyway** for SwiftPDF.
4. If **Open Anyway** does not appear, open Terminal and run:

   ```bash
   xattr -dr com.apple.quarantine /Applications/SwiftPDF.app
   ```

5. Open SwiftPDF again from `Applications`.

This extra step is required because Apple only allows seamless outside-the-App-Store installs for apps signed with a paid Developer ID certificate and notarized by Apple.

## Features

- **Merge** ? combine multiple PDFs into one, reorder pages by drag-and-drop
- **Split** ? extract page ranges into separate documents
- **Sign** ? add signatures to a document and export
- **Preview** ? page thumbnails and full-page rendering

## Development

Prerequisites: [Node.js](https://nodejs.org/) 20+, [Rust](https://rustup.rs/) stable, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```bash
npm install        # install frontend deps
npm run tauri dev  # run the app in dev mode
```

Build a production installer locally:

```bash
npm run tauri build
```

### PDFium

The native PDFium library is bundled per-platform:

- **Windows** ? `src-tauri/resources/pdfium/pdfium.dll` is committed and bundled.
- **macOS** ? `libpdfium.dylib` is fetched in CI from [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries). For a local macOS build, place `libpdfium.dylib` in `src-tauri/resources/pdfium/`.

## Releasing

Pushing a `v*` tag triggers the [release workflow](.github/workflows/release.yml), which builds Windows + macOS (arm64/x64) installers and attaches them to a draft GitHub Release.

```bash
git tag v1.2.3
git push origin v1.2.3
```

## License

MIT
