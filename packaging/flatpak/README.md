# Flatpak packaging (E.5 spike)

Flatpak is the intended supported delivery route (docs/distribution.md) because
it can bundle known WebKitGTK and GStreamer versions instead of depending on
host libraries, which the .deb/.rpm do.

## Layout

- `io.github.zvirox.EasyEditsPro.yml` - the flatpak-builder manifest. Runtime
  `org.gnome.Platform//49` provides WebKitGTK 4.1 (the one library the
  freedesktop runtime lacks); its GStreamer set covers decode (libav, vp8/9,
  opus, isomp4, matroska) and the verified H.264 render path.
- `io.github.zvirox.EasyEditsPro.metainfo.xml` - appstream metadata; validates
  with `appstreamcli validate`.
- `make-payload.sh` - builds `payload/` from the newest (or `$DEB`-given)
  GitHub release .deb using only `ar` + `tar`, matching the acceptance
  runbook's no-dpkg policy.

The payload ships the single dynamic binary and assets that CI already
packages; nothing is compiled inside the sandbox except GStreamer Editing
Services (see below). CI would attach the payload tarball to the release and
the manifest switches the `type: dir` source to a `type: archive` URL.

## Hard-won spike facts (E.5, do not re-learn these)

- **No flatpak runtime ships GStreamer Editing Services.** The engine links
  `libges-1.0`, so the manifest compiles GES into the module; the pinned GES
  minor must track the runtime's GStreamer minor (1.26.x on GNOME 49).
- **libav's `avenc_aac` registers at rank NONE**, and encodebin only considers
  encoders at rank MARGINAL+ for a caps-based profile. Hosts that ship
  faac/vaacenc never notice; a runtime with libav as the only AAC encoder
  fails MP4 export with "Couldn't create encoder for audio/mpeg mpegversion=4".
  Fixed in `saturn-render` (`prefer_software_audio_encoder`), which lifts the
  audio encoder like the hardware-video path already did.
- The sandbox `XDG_RUNTIME_DIR` is a private tmpfs; the MCP bridge socket is
  only host-visible through the full `/run/user/<uid>` bind in the manifest.
- GNOME 48 reached EOL 2026-03; the manifest targets 49.

## Build / install / drive (all user-scoped, no root)

~~~bash
flatpak install --user --or-update flathub org.flatpak.Builder org.gnome.Platform//49 org.gnome.Sdk//49
DEB=path/to/release.deb packaging/flatpak/make-payload.sh
flatpak-builder --user --force-clean --repo spike-repo packaging/flatpak/build packaging/flatpak/io.github.zvirox.EasyEditsPro.yml
flatpak-builder --user --install --force-clean packaging/flatpak/build packaging/flatpak/io.github.zvirox.EasyEditsPro.yml
# Acceptance: same MCP-driven edit-to-render drive as the deb runbook
python3 tests/native/installed_package_workflow.py --flatpak
~~~

## Decision rule (delivery matrix)

Record a launch result or the artifact stays unsupported. If the sandboxed
render cannot be proven, the honest decision is to defer Flatpak past Beta 1
and say so in the release notes - not to ship an unverified route.
