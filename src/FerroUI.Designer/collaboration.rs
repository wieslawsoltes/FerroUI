use ferroui_designer_collaboration::{DocumentReplica, Operation, VersionVector};

#[derive(Clone)]
pub(crate) struct History {
    pub before: String,
    pub after: String,
    pub expected_version: VersionVector,
}
#[derive(Clone)]
pub(crate) struct SessionCollaboration {
    pub replica: DocumentReplica,
    pub outgoing: Vec<Operation>,
    pub undo: Vec<History>,
    pub redo: Vec<History>,
    pub draft_base: Option<String>,
    pub error: Option<String>,
    pub presence_sequence: u64,
}
impl SessionCollaboration {
    pub fn new(replica: DocumentReplica) -> Self {
        Self {
            replica,
            outgoing: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            draft_base: None,
            error: None,
            presence_sequence: 0,
        }
    }
    pub fn local(&mut self, before: String, operation: Operation) {
        self.undo.push(History {
            before,
            after: self.replica.source().to_owned(),
            expected_version: self.replica.version(),
        });
        self.redo.clear();
        self.outgoing.push(operation);
        self.draft_base = None;
        self.error = None;
    }
}
