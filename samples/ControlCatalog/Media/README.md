# Sintel sample video

`sintel-sample.mp4` is a 12-second excerpt (00:02–00:14) from the **Sintel
trailer** by the Durian Open Movie Team, © copyright Blender Foundation | https://durian.blender.org/.
It is licensed under [Creative Commons Attribution 3.0 Unported](https://creativecommons.org/licenses/by/3.0/).
The license permits sharing and adaptation, including commercial reuse, with
attribution. The license text is included as `LICENSE-CC-BY-3.0.txt`.

- Project and download page: https://durian.blender.org/download/
- Original license statement: https://durian.blender.org/sharing/
- Source file: https://download.blender.org/durian/trailer/sintel_trailer-480p.mp4
- Changes: trimmed to 00:02–00:14, scaled to 320×180, converted to Rec.709
  and re-encoded as H.264/AAC with FFmpeg 7.1, optimized for progressive download.
  The untagged source is explicitly interpreted as SMPTE 170M for offline color
  conversion, rather than changing framework frame admission. This is not the full film.
- Size: 423,728 bytes. Sample SHA-256:
  `4465077f2343955806b8865a771cab82ea89f1d1a06f13d4b1af5ec01dcd2820`.
- Downloaded source SHA-256:
  `b670602fa00934ca27c4351bb0efe7ea7a07fae57284e44226025eeed7c51254`.

The catalog displays the author, project URL, license URL and modification
notice beside the player. Retain those credits and this license when
redistributing the sample; this video is not relicensed under FerroUI's license.

Reproduce with FFmpeg and its libx264/AAC encoders (bytes can vary by encoder version):

```sh
curl -fL https://download.blender.org/durian/trailer/sintel_trailer-480p.mp4 -o /tmp/sintel-trailer.mp4
bash samples/ControlCatalog/Media/prepare-sintel.sh /tmp/sintel-trailer.mp4 /tmp/sintel-sample.mp4
```

Native catalog builds embed the small clip. Browser builds copy it to
`media/sintel-sample.mp4` next to the site, outside the catalog asset manifest,
preloads and idle prefetch. It is not included in WebAssembly. Merely opening
the catalog or navigating to Video does not fetch it; **Load bundled video**
opens the same-origin URL through the native browser media element.
