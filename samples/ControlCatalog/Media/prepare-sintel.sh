#!/bin/bash
# MIT. Offline preparation; see README.md for the video's separate CC BY license.
# Input colors are an explicit interpretation of the untagged original source.
set -euo pipefail
ffmpeg -hide_banner -nostdin -n -ss 2 -i "${1:?source trailer MP4}" -t 12 \
  -vf 'colorspace=iall=smpte170m:all=bt709:format=yuv420p,scale=320:180:flags=lanczos' \
  -c:v libx264 -preset medium -b:v 220k -maxrate 300k -bufsize 600k \
  -pix_fmt yuv420p -color_primaries bt709 -color_trc bt709 -colorspace bt709 \
  -c:a aac -b:a 64k -movflags +faststart "${2:?new destination MP4}"
