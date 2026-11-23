//! Tiles made on the spot: a labelled-by-colour grid, for samples and tests that must not reach
//! the network. It is not a map of anything.

use crate::projection::TileId;
use crate::tile_source::{ITileLoader, RasterTileSource, TileFuture};

const SIDE: usize = 256;
const CHUNK: usize = 65_535;

/// The source of [`GridTileLoader`].
pub fn grid_tile_source() -> RasterTileSource {
    RasterTileSource::new("tile://grid/{z}/{x}/{y}", "Sample grid, no map data")
        .expect("the template of the grid is valid")
}

/// Makes each tile as a PNG of a checkerboard with a line along its edges.
#[derive(Clone, Copy, Debug, Default)]
pub struct GridTileLoader;

impl ITileLoader for GridTileLoader {
    fn load(&self, _address: &str, tile: TileId) -> TileFuture {
        Box::pin(async move { Ok(grid_tile(tile)) })
    }
}

fn grid_tile(tile: TileId) -> Vec<u8> {
    let shade: u8 = if (tile.x + tile.y) % 2 == 0 {
        0xDD
    } else {
        0xC8
    };
    let tint = 0x20u8.saturating_add(tile.zoom.saturating_mul(9));
    let mut pixels = Vec::with_capacity((SIDE * 4 + 1) * SIDE);
    for y in 0..SIDE {
        pixels.push(0);
        for x in 0..SIDE {
            let is_edge = x == 0 || y == 0;
            let color = if is_edge {
                [0x70, 0x78, 0x80]
            } else {
                [shade, shade, shade.saturating_sub(tint / 4)]
            };
            pixels.extend_from_slice(&[color[0], color[1], color[2], 0xFF]);
        }
    }
    png(SIDE as u32, SIDE as u32, &pixels)
}

/// A PNG of 8-bit RGBA rows, each with its filter byte, in stored (uncompressed) deflate blocks.
fn png(width: u32, height: u32, rows: &[u8]) -> Vec<u8> {
    let mut stream = vec![0x78, 0x01];
    let mut chunks = rows.chunks(CHUNK).peekable();
    while let Some(block) = chunks.next() {
        stream.push(u8::from(chunks.peek().is_none()));
        let length = block.len() as u16;
        stream.extend_from_slice(&length.to_le_bytes());
        stream.extend_from_slice(&(!length).to_le_bytes());
        stream.extend_from_slice(block);
    }
    stream.extend_from_slice(&adler32(rows).to_be_bytes());

    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);

    let mut file = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    chunk(&mut file, b"IHDR", &header);
    chunk(&mut file, b"IDAT", &stream);
    chunk(&mut file, b"IEND", &[]);
    file
}

fn chunk(file: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    file.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = file.len();
    file.extend_from_slice(kind);
    file.extend_from_slice(data);
    let checksum = crc32(&file[start..]);
    file.extend_from_slice(&checksum.to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for block in data.chunks(5_552) {
        for byte in block {
            a += u32::from(*byte);
            b += a;
        }
        a %= 65_521;
        b %= 65_521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checksums_match_their_published_values() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn a_tile_is_a_well_formed_png() {
        let file = grid_tile(TileId {
            zoom: 3,
            x: 2,
            y: 5,
        });
        assert_eq!(
            &file[..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
        );
        let mut offset = 8;
        let mut kinds = Vec::new();
        let mut stream = Vec::new();
        while offset < file.len() {
            let length = u32::from_be_bytes(file[offset..offset + 4].try_into().unwrap()) as usize;
            let kind = &file[offset + 4..offset + 8];
            let end = offset + 8 + length;
            let stored = u32::from_be_bytes(file[end..end + 4].try_into().unwrap());
            assert_eq!(crc32(&file[offset + 4..end]), stored);
            if kind == b"IHDR" {
                assert_eq!(&file[offset + 8..offset + 16], &[0, 0, 1, 0, 0, 0, 1, 0]);
            }
            if kind == b"IDAT" {
                stream.extend_from_slice(&file[offset + 8..end]);
            }
            kinds.push(kind.to_vec());
            offset = end + 4;
        }
        assert_eq!(
            kinds,
            [b"IHDR".to_vec(), b"IDAT".to_vec(), b"IEND".to_vec()]
        );

        // The stored blocks give back every row.
        let mut rows = Vec::new();
        let mut at = 2;
        loop {
            let is_last = stream[at] == 1;
            let length = u16::from_le_bytes([stream[at + 1], stream[at + 2]]) as usize;
            let check = u16::from_le_bytes([stream[at + 3], stream[at + 4]]);
            assert_eq!(check, !(length as u16));
            rows.extend_from_slice(&stream[at + 5..at + 5 + length]);
            at += 5 + length;
            if is_last {
                break;
            }
        }
        assert_eq!(rows.len(), (SIDE * 4 + 1) * SIDE);
        assert_eq!(
            u32::from_be_bytes(stream[at..at + 4].try_into().unwrap()),
            adler32(&rows)
        );
    }

    #[test]
    fn neighbouring_tiles_differ() {
        let first = grid_tile(TileId {
            zoom: 2,
            x: 0,
            y: 0,
        });
        let second = grid_tile(TileId {
            zoom: 2,
            x: 1,
            y: 0,
        });
        assert_ne!(first, second);
    }
}
