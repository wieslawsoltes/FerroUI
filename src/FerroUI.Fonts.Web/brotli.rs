//! The one place that knows which Brotli decoder WOFF2 uses.

use ferroui_fonts_web_core::woff2::BrotliDecoderFactory;

/// What decodes the Brotli stream of WOFF2 in this build; `None` without the feature `woff2`,
/// and a WOFF2 source is then skipped for the next `src` entry.
pub fn brotli_decoder_factory() -> Option<BrotliDecoderFactory> {
    #[cfg(feature = "woff2")]
    {
        Some(decoder::create)
    }
    #[cfg(not(feature = "woff2"))]
    {
        None
    }
}

#[cfg(feature = "woff2")]
mod decoder {
    use brotli_decompressor::{BrotliDecompressStream, BrotliResult, BrotliState, StandardAlloc};
    use ferroui_fonts_web_core::woff2::{BrotliStatus, IBrotliDecoder};

    struct Decoder {
        state: BrotliState<StandardAlloc, StandardAlloc, StandardAlloc>,
        total_out: usize,
    }

    pub(super) fn create() -> Box<dyn IBrotliDecoder> {
        // The strict state refuses the large-window extension: a stream of the standard format
        // has a window of at most 16 MiB, and a hostile file cannot ask for more.
        let state =
            BrotliState::new_strict(StandardAlloc::default(), StandardAlloc::default(), StandardAlloc::default());
        Box::new(Decoder { state, total_out: 0 })
    }

    impl IBrotliDecoder for Decoder {
        fn decompress(
            &mut self,
            input: &[u8],
            input_offset: &mut usize,
            output: &mut [u8],
            output_offset: &mut usize,
        ) -> BrotliStatus {
            if *input_offset > input.len() || *output_offset > output.len() {
                return BrotliStatus::Failed;
            }
            let mut available_in = input.len() - *input_offset;
            let mut available_out = output.len() - *output_offset;
            let result = BrotliDecompressStream(
                &mut available_in,
                input_offset,
                input,
                &mut available_out,
                output_offset,
                output,
                &mut self.total_out,
                &mut self.state,
            );
            match result {
                BrotliResult::ResultSuccess => BrotliStatus::Finished,
                BrotliResult::NeedsMoreInput => BrotliStatus::NeedsMoreInput,
                BrotliResult::NeedsMoreOutput => BrotliStatus::NeedsMoreOutput,
                BrotliResult::ResultFailure => BrotliStatus::Failed,
            }
        }
    }
}

#[cfg(all(test, feature = "woff2"))]
mod tests {
    use super::*;
    use ferroui_fonts_web_core::sfnt::{sanitize, SanitizeOptions};
    use ferroui_fonts_web_core::woff2::testing::*;
    use ferroui_fonts_web_core::woff2::BrotliStatus;
    use ferroui_fonts_web_core::{decode_font_with_brotli, FontDecodeError, FontDecoder};
    use std::path::PathBuf;

    const ALL: EncodeOptions = EncodeOptions { transform_glyf: true, transform_hmtx: true };

    fn font(path: &str) -> Vec<u8> {
        let full = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(path);
        std::fs::read(&full).unwrap_or_else(|error| panic!("{}: {error}", full.display()))
    }

    fn inter() -> Vec<u8> {
        font("FerroUI.Base/test_assets/fonts/Inter-Regular.ttf")
    }

    /// Decodes a whole stream, writing at most `chunk` bytes at a time.
    fn decompress_in_chunks(stream: &[u8], expected: usize, chunk: usize) -> (Vec<u8>, BrotliStatus) {
        let mut decoder = brotli_decoder_factory().unwrap()();
        let mut output = vec![0u8; expected + 16];
        let (mut input_at, mut written) = (0, 0);
        loop {
            let end = (written + chunk).min(output.len());
            let mut step = 0;
            let status = decoder.decompress(stream, &mut input_at, &mut output[written..end], &mut step);
            written += step;
            if !matches!(status, BrotliStatus::NeedsMoreOutput) {
                output.truncate(written);
                return (output, status);
            }
        }
    }

    #[test]
    fn the_decoder_reads_stored_streams_at_any_output_size() {
        let data: Vec<u8> = (0..200_000u32).map(|value| (value ^ (value >> 7)) as u8).collect();
        let stream = stored_brotli(&data);
        for chunk in [1, 7, 4096, 65536, 1 << 20] {
            let (output, status) = decompress_in_chunks(&stream, data.len(), chunk);
            assert_eq!(status, BrotliStatus::Finished, "chunk {chunk}");
            assert!(output == data, "chunk {chunk}");
        }
    }

    #[test]
    fn the_decoder_resumes_when_the_input_comes_in_pieces() {
        let data: Vec<u8> = (0..70_000u32).map(|value| (value * 31) as u8).collect();
        let stream = stored_brotli(&data);
        let mut decoder = brotli_decoder_factory().unwrap()();
        let mut output = vec![0u8; data.len()];
        let (mut written, mut given) = (0, 0);
        let mut last = BrotliStatus::NeedsMoreInput;
        while given < stream.len() {
            let start = given;
            given = (given + 997).min(stream.len());
            let piece = &stream[start..given];
            let (mut input_at, mut step) = (0, 0);
            last = decoder.decompress(piece, &mut input_at, &mut output[written..], &mut step);
            written += step;
            assert_ne!(last, BrotliStatus::Failed);
            assert_eq!(input_at, piece.len(), "the decoder keeps what it cannot use yet");
        }
        assert_eq!(last, BrotliStatus::Finished);
        assert!(output == data);
    }

    #[test]
    fn an_empty_stream_decodes_to_nothing() {
        let (output, status) = decompress_in_chunks(&stored_brotli(&[]), 0, 16);
        assert_eq!(status, BrotliStatus::Finished);
        assert!(output.is_empty());
    }

    #[test]
    fn a_broken_stream_fails() {
        // ISLAST = 0, MNIBBLES = 3 announces a metadata block, with a reserved bit set.
        let (_, status) = decompress_in_chunks(&[0b0000_1110, 0xFF, 0xFF, 0xFF], 16, 16);
        assert_ne!(status, BrotliStatus::Finished);
    }

    #[test]
    fn woff2_decodes_end_to_end() {
        let data = inter();
        let options = SanitizeOptions::default();
        let file = encode_woff2(&data, ALL, &stored_brotli);
        let decoded = decode_font_with_brotli(&file, &options, brotli_decoder_factory()).unwrap();
        let clean = sanitize(&data, &options).unwrap();
        assert_eq!(outlines(&decoded), outlines(&clean));
        assert_eq!(font_table(&decoded, b"hmtx"), font_table(&clean, b"hmtx"));
        assert_eq!(font_table(&decoded, b"GSUB"), font_table(&clean, b"GSUB"));
    }

    #[test]
    fn woff2_decodes_the_same_in_small_steps() {
        let data = inter();
        let options = SanitizeOptions::default();
        let file = encode_woff2(&data, ALL, &stored_brotli);
        let whole = decode_font_with_brotli(&file, &options, brotli_decoder_factory()).unwrap();
        let mut decoder = FontDecoder::with_brotli(&file, options, brotli_decoder_factory()).unwrap();
        let mut steps = 0;
        let stepped = loop {
            steps += 1;
            if let Some(font) = decoder.step(4096).unwrap() {
                break font;
            }
        };
        assert!(steps > 16, "the work is spread over steps");
        assert!(whole == stepped);
    }

    #[test]
    fn a_stream_that_writes_more_or_less_than_declared_is_refused() {
        let data = font("Headless/FerroUI.Headless/BareMinimum.ttf");
        let options = SanitizeOptions::default();
        for delta in [-1isize, 1] {
            let file = encode_woff2(&data, ALL, &|stream| {
                let mut changed = stream.to_vec();
                if delta < 0 {
                    changed.pop();
                } else {
                    changed.push(0);
                }
                stored_brotli(&changed)
            });
            let result = decode_font_with_brotli(&file, &options, brotli_decoder_factory());
            assert_eq!(result.err(), Some(FontDecodeError::Malformed("WOFF2 compressed stream")), "{delta}");
        }
    }

    /// Round trips fonts through really compressed streams. `FERROUI_BROTLI_COMPRESS` is a
    /// command that compresses its standard input to its standard output, as Brotli.
    /// `FERROUI_WOFF2_OUT`, when set, is a directory that receives the files, for a check with
    /// another decoder.
    #[test]
    #[ignore = "needs a Brotli compressor"]
    fn woff2_with_real_compression() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let tool = std::env::var("FERROUI_BROTLI_COMPRESS").expect("FERROUI_BROTLI_COMPRESS");
        let compress = |stream: &[u8]| -> Vec<u8> {
            let mut child =
                Command::new(&tool).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().expect("the compressor");
            let mut stdin = child.stdin.take().unwrap();
            let input = stream.to_vec();
            let writer = std::thread::spawn(move || stdin.write_all(&input));
            let output = child.wait_with_output().unwrap();
            writer.join().unwrap().unwrap();
            assert!(output.status.success());
            output.stdout
        };
        let options = SanitizeOptions::default();
        for path in [
            "FerroUI.Base/test_assets/fonts/Inter-Regular.ttf",
            "FerroUI.Base/test_assets/fonts/InterVariable.ttf",
            "FerroUI.Themes.Material/Assets/Roboto-Regular.ttf",
            "../samples/ControlCatalog/Assets/Fonts/SourceSansPro-Regular.ttf",
            "../samples/ControlCatalog/Assets/FallbackFonts/NotoSansArabic-Catalog.ttf",
            "FerroUI.Base/test_assets/fonts/WenQuanYiMicroHei-Subset.ttf",
        ] {
            let data = font(path);
            let file = encode_woff2(&data, ALL, &compress);
            let clean = sanitize(&data, &options).unwrap();
            let whole = decode_font_with_brotli(&file, &options, brotli_decoder_factory()).unwrap();
            assert_eq!(outlines(&whole), outlines(&clean), "{path}");
            let mut decoder = FontDecoder::with_brotli(&file, options, brotli_decoder_factory()).unwrap();
            let stepped = loop {
                if let Some(font) = decoder.step(1024).unwrap() {
                    break font;
                }
            };
            assert!(whole == stepped, "{path}");
            let name = path.rsplit('/').next().unwrap().replace(".ttf", "");
            println!("{name}: sfnt {} bytes, woff2 {} bytes", data.len(), file.len());
            if let Ok(directory) = std::env::var("FERROUI_WOFF2_OUT") {
                std::fs::write(format!("{directory}/{name}.ttf"), &clean).unwrap();
                for (suffix, transform_glyf, transform_hmtx) in
                    [("gh", true, true), ("g", true, false), ("plain", false, false)]
                {
                    let variant = encode_woff2(&data, EncodeOptions { transform_glyf, transform_hmtx }, &compress);
                    std::fs::write(format!("{directory}/{name}.{suffix}.woff2"), variant).unwrap();
                }
            }
        }
    }

    #[test]
    fn corrupt_files_never_panic() {
        let data = font("Headless/FerroUI.Headless/BareMinimum.ttf");
        let file = encode_woff2(&data, ALL, &stored_brotli);
        let options = SanitizeOptions::default();
        let mut state = 0xA076_1D64_78BD_642Fu64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as usize
        };
        for _ in 0..2000 {
            let mut copy = file.clone();
            for _ in 0..4 {
                let position = next() % copy.len();
                copy[position] = next() as u8;
            }
            let _ = decode_font_with_brotli(&copy, &options, brotli_decoder_factory());
        }
    }
}

#[cfg(all(test, not(feature = "woff2")))]
mod tests_without_the_feature {
    #[test]
    fn there_is_no_decoder() {
        assert!(super::brotli_decoder_factory().is_none());
    }
}
