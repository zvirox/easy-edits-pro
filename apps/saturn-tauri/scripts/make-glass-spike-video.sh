#!/usr/bin/env bash
# Generates the moving test clip the glass spike plays behind its panels.
# Not committed: it is a build artifact of the spike, not an app asset.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="${here}/../ui/public/glass-spike.mp4"

command -v gst-launch-1.0 >/dev/null 2>&1 || {
  echo "gst-launch-1.0 not found (install gstreamer)" >&2
  exit 1
}
gst-inspect-1.0 x264enc >/dev/null 2>&1 || {
  echo "x264enc not found (install gst-plugins-ugly)" >&2
  exit 1
}

mkdir -p "$(dirname "${out}")"

gst-launch-1.0 -q \
  videotestsrc pattern=ball num-buffers=360 ! \
  video/x-raw,width=1280,height=720,framerate=30/1 ! \
  x264enc tune=zerolatency speed-preset=veryfast bitrate=6000 key-int-max=30 ! \
  h264parse ! \
  mp4mux ! \
  filesink location="${out}"

echo "wrote ${out}"
