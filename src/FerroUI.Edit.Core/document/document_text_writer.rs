use super::{DocumentError, TextDocument, TextUtilities};

/// A UTF-16-aware writer that inserts directly into a shared document. Keep a
/// document update guard alive to combine multiple writes into one undo step.
#[derive(Clone, Debug)]
pub struct DocumentTextWriter {
    document: TextDocument,
    insertion_offset: usize,
    new_line: String,
}

impl DocumentTextWriter {
    pub fn new(document: TextDocument, insertion_offset: usize) -> Result<Self, DocumentError> {
        let line = document.get_line_by_offset(insertion_offset)?;
        let new_line = TextUtilities::get_new_line_from_document(&document, line.line_number)?;
        Ok(Self {
            document,
            insertion_offset,
            new_line,
        })
    }
    pub fn insertion_offset(&self) -> usize {
        self.insertion_offset
    }
    pub fn set_insertion_offset(&mut self, offset: usize) -> Result<(), DocumentError> {
        self.document.get_line_by_offset(offset)?;
        self.insertion_offset = offset;
        Ok(())
    }
    pub fn new_line(&self) -> &str {
        &self.new_line
    }
    pub fn set_new_line(&mut self, value: impl Into<String>) {
        self.new_line = value.into();
    }
    pub fn write(&mut self, text: &str) -> Result<(), DocumentError> {
        self.document.insert(self.insertion_offset, text)?;
        self.insertion_offset += text.encode_utf16().count();
        Ok(())
    }
    pub fn write_char(&mut self, value: char) -> Result<(), DocumentError> {
        self.write(value.encode_utf8(&mut [0; 4]))
    }
    pub fn write_utf16(&mut self, text: &[u16]) -> Result<(), DocumentError> {
        self.document
            .replace_utf16(self.insertion_offset, 0, text)?;
        self.insertion_offset += text.len();
        Ok(())
    }
    pub fn write_line(&mut self, text: &str) -> Result<(), DocumentError> {
        let _group = self.document.begin_update()?;
        self.write(text)?;
        self.write(&self.new_line.clone())
    }
}

impl std::fmt::Write for DocumentTextWriter {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        self.write(value).map_err(|_| std::fmt::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_newline_and_advances_by_utf16_units() {
        let document = TextDocument::from_text("x\r\n");
        let mut writer = DocumentTextWriter::new(document.clone(), 3).unwrap();
        assert_eq!(writer.new_line(), "\r\n");
        writer.write_line("😀").unwrap();
        assert_eq!(writer.insertion_offset(), 7);
        assert_eq!(document.text(), "x\r\n😀\r\n");
        document.undo().unwrap();
        assert_eq!(document.text(), "x\r\n");
    }
}
