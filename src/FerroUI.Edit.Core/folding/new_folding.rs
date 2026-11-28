#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewFolding {
    pub start_offset: usize,
    pub end_offset: usize,
    pub name: Option<String>,
    pub default_closed: bool,
    pub is_definition: bool,
}
impl NewFolding {
    pub fn new(start_offset: usize, end_offset: usize) -> Self {
        Self {
            start_offset,
            end_offset,
            ..Self::default()
        }
    }
    pub fn length(&self) -> usize {
        self.end_offset.saturating_sub(self.start_offset)
    }
}
