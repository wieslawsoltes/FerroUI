//! One build artifact for the instruction streams of a generated Rust module.
//! The generated static imports and functions still form the linker reachability graph.
use std::{cell::RefCell, collections::HashMap, fmt::Write, rc::Rc};

#[derive(Default)]
struct Packed {
    bytes: Vec<u8>,
    offsets: HashMap<Vec<u8>, usize>,
}

/// An in-memory emitter defaults to self-contained Rust byte literals. Cargo creates a
/// packed image per module and writes its bytes alongside the generated source.
#[derive(Clone, Default)]
pub struct BytecodeImage(Option<Rc<RefCell<Packed>>>);

impl BytecodeImage {
    pub fn packed() -> Self {
        Self(Some(Rc::default()))
    }

    pub(super) fn program(&self, code: &[u8]) -> String {
        match &self.0 {
            Some(image) => {
                let mut image = image.borrow_mut();
                let offset = match image.offsets.get(code) {
                    Some(offset) => *offset,
                    None => {
                        let offset = image.bytes.len();
                        image.bytes.extend_from_slice(code);
                        image.offsets.insert(code.to_owned(), offset);
                        offset
                    }
                };
                format!(
                    "&const {{ rt::program_bytes::<{}>(__FERROUI_XAML_IMAGE, {offset}) }}",
                    code.len()
                )
            }
            None => {
                let mut literal = String::from("b\"");
                for byte in code {
                    write!(literal, "\\x{byte:02x}").unwrap();
                }
                literal.push('"');
                literal
            }
        }
    }

    /// Returns the bytes and the static to append after the module's generated source.
    /// Appending keeps source-map line numbers stable.
    pub fn artifact(&self, file_name: &str) -> Option<(Vec<u8>, String)> {
        let image = self.0.as_ref()?.borrow();
        if image.offsets.is_empty() {
            return None;
        }
        let file_name = super::source::rust_string_literal(file_name);
        Some((
            image.bytes.clone(),
            format!("\nconst __FERROUI_XAML_IMAGE: &[u8] = include_bytes!({file_name});\n"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_packed_module_reuses_stream_offsets_and_embeds_each_program_by_value() {
        let image = BytecodeImage::packed();
        assert!(image.artifact("app.fxc").is_none());
        let first = image.program(&[1, 4, 8]);
        assert_eq!(
            first,
            "&const { rt::program_bytes::<3>(__FERROUI_XAML_IMAGE, 0) }"
        );
        assert_eq!(image.clone().program(&[1, 4, 8]), first);
        assert_eq!(
            image.program(&[3]),
            "&const { rt::program_bytes::<1>(__FERROUI_XAML_IMAGE, 3) }"
        );
        let (data, declaration) = image.artifact("app.fxc").unwrap();
        assert_eq!(data, [1, 4, 8, 3]);
        assert!(declaration.contains("include_bytes!(\"app.fxc\")"));
        assert_eq!(
            BytecodeImage::default().program(&[1, 255]),
            "b\"\\x01\\xff\""
        );
    }
}
