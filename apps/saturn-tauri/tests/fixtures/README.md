# Editor smoke-test media

`editor-smoke.webm` is a six-second synthetic color-pattern video generated with FFmpeg's `testsrc2` source and VP8 encoder. It contains no third-party footage and is used only by the Playwright UI smoke test.

To regenerate it, run:

```sh
ffmpeg -hide_banner -loglevel error -f lavfi -i 'testsrc2=size=640x360:rate=30:duration=6' -c:v libvpx -b:v 600k -an -y editor-smoke.webm
```
