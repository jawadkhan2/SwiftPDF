# SwiftPDF — Agent Guide

Tauri (Rust) + SvelteKit desktop PDF app. PDF ops via PDFium (`pdfium-render`).

## Release runbook

Releases are built by CI, **not** locally. macOS installers cannot be built on Windows.

### How it works

- Workflow: `.github/workflows/release.yml`. Triggers on push of a `v*` tag (or manual `workflow_dispatch` with a `tag` input).
- Matrix builds 3 jobs: Windows (`windows-latest`), macOS arm64 + macOS x64 (both `macos-latest`).
- `tauri-apps/tauri-action` builds and attaches artifacts to a **draft** GitHub Release (`releaseDraft: true`). A human/agent publishes after verifying.
- Artifacts per release: Windows `.msi` + `.exe`, macOS `.dmg` (arm64 + x64), plus `.app.tar.gz` auto-update bundles.

### Steps to cut a release

1. Bump version in **all four** spots (must match):
   - `package.json` → `version`
   - `src-tauri/Cargo.toml` → `[package] version`
   - `src-tauri/Cargo.lock` → the `tauri-app` package entry `version`
   - `src-tauri/tauri.conf.json` → `version`
2. Commit + push to `main`.
3. Tag and push (this triggers CI):
   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
4. Watch the run:
   ```bash
   gh run list --repo jawadkhan2/SwiftPDF --limit 1
   gh run view <run-id> --repo jawadkhan2/SwiftPDF --json status,conclusion,jobs \
     -q '.status, (.jobs[] | "\(.name): \(.status) \(.conclusion // "")")'
   ```
   Cold Rust build ≈ 10–20 min per job.
5. After all jobs succeed, verify artifacts on the draft:
   ```bash
   gh release view vX.Y.Z --repo jawadkhan2/SwiftPDF --json isDraft,assets \
     -q '.isDraft, (.assets[] | "\(.name) \(.size)")'
   ```
   Expect 4 installers (.msi, .exe, 2× .dmg) + 2 .app.tar.gz.
6. Publish:
   ```bash
   gh release edit vX.Y.Z --repo jawadkhan2/SwiftPDF --draft=false
   ```

### PDFium (the cross-platform gotcha)

`src-tauri/src/pdf/engine.rs` loads the native PDFium library at runtime from the bundled resource dir (`resources/pdfium/`). The library is bundled **per-platform** via Tauri's platform config merge:

- `src-tauri/tauri.windows.conf.json` → bundles `resources/pdfium/pdfium.dll` (committed to repo).
- `src-tauri/tauri.macos.conf.json` → bundles `resources/pdfium/libpdfium.dylib`, downloaded in CI from [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) (latest release).
- Base `tauri.conf.json` has **no** `bundle.resources` — keep it that way; resources are platform-specific.

If adding a platform, supply the matching PDFium binary + a `tauri.<platform>.conf.json` resource entry, or the app builds but fails at runtime on PDF ops.

### Caveats

- Builds are **unsigned** (no Apple/Windows certs). Users hit Gatekeeper/SmartScreen warnings. Signing requires paid certs + CI secrets.
- macOS builds cannot be runtime-tested from a Windows dev box. Test on real hardware when possible.

## Dev

```bash
npm install
npm run tauri dev    # run app
npm run tauri build  # local installer (current platform only)
npm run check        # svelte-check / typecheck
```
