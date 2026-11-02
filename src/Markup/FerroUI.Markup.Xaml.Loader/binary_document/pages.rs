//! FXP v3 partitions an installed revision into independently comparable frames.
//! A frame is one complete FXP v2 image. File contents do not determine frame
//! boundaries, so editing a member cannot shift otherwise identical frames.
use super::{
    invalid, solid, take, uint, ThemePack, MAGIC, MAX_DOCUMENTS, MAX_PACK_BYTES, PACK_VERSION,
};
use xamlx::exceptions::XamlResult;

const VERSION: u8 = 3;
const DOCUMENTS_PER_PAGE: usize = 16;
const INDEX_HEADER: usize = 13;

pub(super) fn is_paged(image: &[u8]) -> bool {
    image.starts_with(MAGIC) && image.get(4) == Some(&VERSION)
}

pub fn compile_paged_documents(documents: &[(&str, &str)]) -> XamlResult<Vec<u8>> {
    if documents.is_empty() || documents.len() > MAX_DOCUMENTS {
        return Err(invalid("invalid document count"));
    }
    let mut sorted = documents.to_vec();
    sorted.sort_unstable_by_key(|&(path, _)| path);
    if sorted.windows(2).any(|p| p[0].0 == p[1].0) {
        return Err(invalid("duplicate path"));
    }
    let mut output = MAGIC.to_vec();
    output.push(VERSION);
    output.extend_from_slice(&0u32.to_le_bytes());
    let mut count = 0u32;
    let mut expanded = INDEX_HEADER;
    for directory in sorted.chunk_by(|a, b| {
        a.0.rsplit_once('/').map(|(d, _)| d) == b.0.rsplit_once('/').map(|(d, _)| d)
    }) {
        for docs in directory.chunks(DOCUMENTS_PER_PAGE) {
            let (image, size) = solid::compile_page(docs)?;
            expanded = add_page(expanded, size)?;
            if output.len() + 8 + image.len() > MAX_PACK_BYTES {
                return Err(invalid("paged archive too large"));
            }
            output.extend_from_slice(&(size as u32).to_le_bytes());
            output.extend_from_slice(&(image.len() as u32).to_le_bytes());
            output.extend_from_slice(&image);
            count += 1;
        }
    }
    output[5..9].copy_from_slice(&count.to_le_bytes());
    Ok(output)
}

fn add_page(total: usize, size: usize) -> XamlResult<usize> {
    size.checked_sub(INDEX_HEADER)
        .and_then(|n| total.checked_add(n))
        .filter(|&n| n <= MAX_PACK_BYTES)
        .ok_or_else(|| invalid("invalid expanded page size"))
}

#[derive(PartialEq)]
struct Page<'a> {
    frame: &'a [u8],
    expanded: usize,
}

impl Page<'_> {
    fn open(&self) -> XamlResult<Vec<u8>> {
        let indexed = solid::expand_solid(self.frame)?;
        if indexed.len() != self.expanded {
            return Err(invalid("page expansion length mismatch"));
        }
        Ok(indexed)
    }
}

fn pages(image: &[u8]) -> XamlResult<Vec<Page<'_>>> {
    if !is_paged(image) || image.len() > MAX_PACK_BYTES {
        return Err(invalid("invalid paged envelope"));
    }
    let mut bytes = &image[5..];
    let count = uint(&mut bytes)?;
    if count == 0 || count > MAX_DOCUMENTS || count > bytes.len() / 17 {
        return Err(invalid("invalid page count"));
    }
    let mut pages = Vec::with_capacity(count);
    let mut total = INDEX_HEADER;
    for _ in 0..count {
        let expanded = uint(&mut bytes)?;
        total = add_page(total, expanded)?;
        let length = uint(&mut bytes)?;
        let frame = take(&mut bytes, length)?;
        if !solid::is_solid(frame) {
            return Err(invalid("invalid page frame"));
        }
        pages.push(Page { frame, expanded });
    }
    if !bytes.is_empty() {
        return Err(invalid("trailing page data"));
    }
    Ok(pages)
}

pub(super) fn expand(image: &[u8]) -> XamlResult<Vec<u8>> {
    let mut index = Vec::new();
    let mut data = Vec::new();
    let mut count = 0usize;
    for page in pages(image)? {
        let owned = page.open()?;
        // The inner reader already checked this v1 directory and its members.
        let mut tail = &owned[5..];
        count += uint(&mut tail)?;
        if count > MAX_DOCUMENTS {
            return Err(invalid("too many paged documents"));
        }
        let index_len = uint(&mut tail)?;
        index.extend_from_slice(take(&mut tail, index_len)?);
        data.extend_from_slice(tail);
    }
    let mut out = Vec::with_capacity(INDEX_HEADER + index.len() + data.len());
    out.extend_from_slice(MAGIC);
    out.push(PACK_VERSION);
    out.extend_from_slice(&(count as u32).to_le_bytes());
    out.extend_from_slice(&(index.len() as u32).to_le_bytes());
    out.extend(index);
    out.extend(data);
    // Checks ordering and uniqueness across page boundaries.
    ThemePack::open(&out)?;
    Ok(out)
}

/// Compare against a known-good installed revision. Equal frames are proven by
/// byte equality. Only unequal frames are expanded. Some returns complete old
/// and new FXD images for each changed path. None requires ordinary loading
/// (a different format or path inventory). Callers validate the syntax and
/// semantics of changed documents before applying any edit.
pub fn changed_documents(
    installed: &[u8],
    replacement: &[u8],
) -> XamlResult<Option<Vec<(String, Vec<u8>, Vec<u8>)>>> {
    if !is_paged(installed) || !is_paged(replacement) {
        return Ok(None);
    }
    let original = pages(installed)?;
    let incoming = pages(replacement)?;
    if original.len() != incoming.len() {
        return Ok(None);
    }
    let mut changed = Vec::new();
    for (old, new) in original.iter().zip(&incoming) {
        if old == new {
            continue;
        }
        let old = old.open()?;
        let new = new.open()?;
        let a = ThemePack::open(&old)?;
        let b = ThemePack::open(&new)?;
        if !a.paths().eq(b.paths()) {
            return Ok(None);
        }
        for (&(path, before), &(_, after)) in a.entries.iter().zip(&b.entries) {
            if before != after {
                changed.push((path.to_string(), before.to_vec(), after.to_vec()));
            }
        }
    }
    Ok(Some(changed))
}

#[cfg(test)]
mod tests {
    use super::*;
    const BASE: &[(&str, &str)] = &[("themes/a.xaml", "<Panel/>"), ("other/b.xaml", "<Button/>")];

    #[test]
    fn indexed_documents_survive_edits_and_repartitioning() {
        let a = compile_paged_documents(BASE).unwrap();
        let b =
            compile_paged_documents(&[("other/b.xaml", "<Button/>"), ("themes/a.xaml", "<Text/>")])
                .unwrap();
        let flat = expand(&a).unwrap();
        let solid = solid::compile_solid_documents(BASE).unwrap();
        assert_eq!(flat, solid::expand_solid(&solid).unwrap());
        assert_eq!(a, compile_paged_documents(&[BASE[1], BASE[0]]).unwrap());
        assert!(changed_documents(&a, &solid).unwrap().is_none());
        assert!(changed_documents(&a, &a).unwrap().unwrap().is_empty());
        let delta = changed_documents(&a, &b).unwrap().unwrap();
        assert_eq!(delta.len(), 1);
        let (path, old, new) = &delta[0];
        assert_eq!(path, "themes/a.xaml");
        assert_eq!(old, ThemePack::open(&flat).unwrap().document(path).unwrap());
        assert_eq!(
            new,
            ThemePack::open(&expand(&b).unwrap())
                .unwrap()
                .document(path)
                .unwrap()
        );
        let rename = compile_paged_documents(&[("themes/c.xaml", "<Panel/>"), BASE[1]]).unwrap();
        assert!(changed_documents(&a, &rename).unwrap().is_none());
        assert!(compile_paged_documents(&[BASE[0], BASE[0]]).is_err());
    }

    #[test]
    fn reject_corrupt_and_overlapping_pages() {
        let good = compile_paged_documents(BASE).unwrap();
        for n in 0..good.len() {
            assert!(expand(&good[..n]).is_err());
        }
        for offset in [5, 9, 13] {
            let mut bad = good.clone();
            bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(expand(&bad).is_err());
            assert!(changed_documents(&good, &bad).is_err());
        }
        let mut corrupt = good.clone();
        *corrupt.last_mut().unwrap() ^= 0x80;
        assert!(changed_documents(&good, &corrupt).is_err());
        let mut overlap = compile_paged_documents(&BASE[..1]).unwrap();
        overlap[5] = 2;
        overlap.extend_from_within(9..);
        assert!(expand(&overlap).is_err());
    }
}
