# Release & Packaging Audit — Easy Edits Pro

**Audited:** 2026-11-24  
**Auditor:** Prakash / Antigravity  
**Repository:** `easy-edits-pro` (`https://github.com/zvirox/easy-edits-pro`)

---

## 1. Workflows in `.github/workflows/`

* **`.github/workflows/ci.yml`**:
  * **Trigger:** `push` on `main`, `pull_request` on `main`.
  * **Jobs:** Format, clippy, unit tests, frontend build, source size gate.
* **`.github/workflows/linux-packages.yml`**:
  * **Trigger:** `push` on tags matching `v*-alpha.*`, `v*-beta.*`, `v*-beta`, or `workflow_dispatch`.
  * **Jobs:**
    1. `packages`: builds Debian (`.deb`) and RPM (`.rpm`) using `pnpm tauri build`.
    2. `install-proof-deb`: verifies clean container install on Debian 12.
    3. `install-proof-rpm`: verifies clean container install on Fedora latest.
    4. `install-proof-flatpak`: builds GNOME 49 Flatpak bundle `easy-edits-pro-linux-x86_64.flatpak` and attaches to release.

---

## 2. Existing Packaging Files & Configuration

* **`apps/saturn-tauri/tauri.conf.json`**:
  * `"productName": "Easy Edits Pro"`
  * `"bundle": { "active": true, "targets": ["deb", "rpm"], ... }`
* **`packaging/` directory contents:**
  * `packaging/flatpak/io.github.zvirox.EasyEditsPro.yml`
  * `packaging/flatpak/make-payload.sh`
  * `packaging/flatpak/payload/...`
  * `packaging/flatpak/spike-repo/...`
  * `assets/icons/...`
  * `scripts/prepare-bundle-inputs.sh`

---

## 3. Existing Tags and Release Assets

* **Tags:** `v0.1.0-alpha.1` through `v0.1.2-alpha.8`, `v0.1.3-beta.1`, `v0.1.3-beta.2`
* **Latest Release (`v0.1.3-beta.2`):**
  * `Easy.Edits.Pro_0.1.3-beta.2_amd64.deb` (24.56 MiB)
  * `Easy.Edits.Pro-0.1.3-beta.2-1.x86_64.rpm` (24.64 MiB)
  * `easy-edits-pro-linux-x86_64.flatpak` (15.85 MiB)
  * `easy-edits-pro-linux-x86_64.flatpak.sha256` (102 B)
  * `SHA256SUMS` (219 B)
  * *(Notice: No Arch `pkg.tar.zst` was built; dot-separated `Easy.Edits.Pro` naming was emitted by Tauri)*

---

## 4. Version Alignment Check

| Location | Version | Matches Latest Tag (`v0.1.3-beta.2`)? |
|---|---|---|
| `apps/saturn-tauri/Cargo.toml` | `0.1.0` | **Mismatch** (`0.1.0` vs `v0.1.3-beta.2`) |
| `apps/saturn-tauri/package.json` | `0.1.0` | **Mismatch** |
| `apps/saturn-tauri/tauri.conf.json` | `0.1.0` | **Mismatch** (Workflow injected tag version dynamically) |

---

## 5. Naming Matrix & Identifier Alignment

* **GitHub Repository Name:** `easy-edits-pro`
* **Cargo Binary Name:** `saturn-tauri` (`apps/saturn-tauri/Cargo.toml`: `[package] name = "saturn-tauri"`)
* **Installed Main Binary:** Currently `/usr/bin/saturn-tauri` (or `/usr/bin/easy-edits-pro`)
* **Helper Binaries:** `/usr/bin/saturn-plugin-worker`
* **Tauri `productName`:** `"Easy Edits Pro"`
* **Tauri Identifier:** `com.zvirox.easyeditspro`
* **Mismatch Assessment:**
  - Repo name is `easy-edits-pro`, but Cargo package is `saturn-tauri`.
  - Under Rule 1, the binary should provide `/usr/bin/easy-edits-pro`. Adding `[[bin]] name = "easy-edits-pro" path = "src/main.rs"` cleanly produces the exact expected binary name while keeping internal crate structure intact.

---

## 6. Runtime Dependencies & Extra Files

* **Binaries:**
  * `/usr/bin/easy-edits-pro`
  * `/usr/bin/saturn-plugin-worker`
* **Desktop File:** `/usr/share/applications/easy-edits-pro.desktop`
* **Icons:** `/usr/share/icons/hicolor/<size>/apps/easy-edits-pro.png`
* **Runtime Dependencies:**
  * Debian/Ubuntu: `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libgstreamer1.0-0`, `libgstreamer-plugins-base1.0-0`, `libges-1.0-0`, `gstreamer1.0-plugins-good`, `gstreamer1.0-plugins-ugly`, `gstreamer1.0-plugins-bad`, `gstreamer1.0-libav`
  * Fedora/RPM: `webkit2gtk4.1`, `gtk3`, `gstreamer1`, `gstreamer1-plugins-base`, `gstreamer1-plugins-good`, `gstreamer1-plugins-bad-free`, `gst-editing-services`
  * Arch Linux: `webkit2gtk-4.1`, `gtk3`, `gstreamer`, `gst-plugins-base`, `gst-plugins-good`, `gst-plugins-bad`, `gst-plugins-ugly`, `gst-editing-services`

---

## 7. Secrets Used

* **`GITHUB_TOKEN`**: Standard repository write token.

---

## 8. Packaging Formats & Migration Action Table

| Format | Present | Job / File Location | Action |
|---|---|---|---|
| **Debian (.deb)** | Yes | `.github/workflows/linux-packages.yml` | **Replace** with unified `nfpm` workflow |
| **RPM (.rpm)** | Yes | `.github/workflows/linux-packages.yml` | **Replace** with unified `nfpm` workflow |
| **Arch (.pkg.tar.zst)** | No | None | **Add** via unified `nfpm` workflow |
| **Flatpak** | Yes | `.github/workflows/linux-packages.yml` & `packaging/flatpak/` | **Delete** job & `packaging/flatpak/` (stops Flatpak build bloat) |
| **Checksums** | Yes | `SHA256SUMS` in release | **Keep** & unify with signature generation |
