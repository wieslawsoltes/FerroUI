`qr-multiple.pgm` is a generated 512×256 grayscale, analysis-only fixture. It
contains two QR symbols encoding `ferroui-native-qr-left` and
`ferroui-native-qr-right`, centered at normalized x=0.25 and x=0.75. It contains
no application assets or personal information. The generator is the Apple QR
provider's native-engine contract test using the optional Rust encoder:

```sh
FERROUI_QR_FIXTURE_PGM="$PWD/tests/media/fixtures/qr-multiple.pgm" \
  cargo test -p ferroui-media-qr-apple --lib
```

Production native QR dependencies do not include this encoder or the Rust
decoder. CPU-created fixture storage is not admitted as GPU preview evidence.
