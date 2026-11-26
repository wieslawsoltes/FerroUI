//! Retain recent semantic token results for LSP delta requests. Tokenization stays in the
//! portable language service and each delta is relative to one result for the same document.
use super::LspServer;
use crate::{Result, SemanticToken};
use serde_json::{Value, json};
use std::collections::VecDeque;

const MAX_RESULTS: usize = 16;
const MAX_RESULT_INTEGERS: usize = 128_000;

#[derive(Default)]
pub(super) struct TokenResults {
    serial: u64,
    recent: VecDeque<TokenResult>,
}

struct TokenResult {
    uri: String,
    id: String,
    data: Vec<u32>,
}

impl LspServer {
    pub(super) fn semantic_tokens(&mut self, uri: &str, previous: Option<&str>) -> Result<Value> {
        let identity = self.service.document_uri(uri)?.into_owned();
        let data = encode(self.service.semantic_tokens(uri)?);
        Ok(self.tokens.result(identity, data, previous))
    }
}

impl TokenResults {
    pub(super) fn close(&mut self, uri: &str) {
        self.recent.retain(|result| result.uri != uri);
    }

    fn result(&mut self, uri: String, data: Vec<u32>, previous: Option<&str>) -> Value {
        if data.len() > MAX_RESULT_INTEGERS {
            return json!({ "data": data });
        }
        self.serial = self.serial.wrapping_add(1);
        let id = self.serial.to_string();
        let edits = previous
            .and_then(|id| {
                self.recent
                    .iter()
                    .find(|old| old.id == id && old.uri == uri)
            })
            .and_then(|old| delta(&old.data, &data));
        let result = match edits {
            Some(edits) => json!({ "resultId": id, "edits": edits }),
            None => json!({ "resultId": id, "data": data }),
        };
        if self.recent.len() == MAX_RESULTS {
            self.recent.pop_front();
        }
        self.recent.push_back(TokenResult { uri, id, data });
        result
    }
}

fn encode(tokens: Vec<SemanticToken>) -> Vec<u32> {
    let mut data = Vec::with_capacity(tokens.len() * 5);
    let mut line = 0;
    let mut start = 0;
    for token in tokens {
        data.extend([
            token.line - line,
            if token.line == line {
                token.start - start
            } else {
                token.start
            },
            token.length,
            token.token_type,
            token.modifiers,
        ]);
        line = token.line;
        start = token.start;
    }
    data
}

fn delta(old: &[u32], new: &[u32]) -> Option<Vec<Value>> {
    let prefix = old
        .iter()
        .zip(new)
        .take_while(|(left, right)| left == right)
        .count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(left, right)| left == right)
        .count();
    let removed = old.len() - prefix - suffix;
    let inserted = &new[prefix..new.len() - suffix];
    if removed == 0 && inserted.is_empty() {
        return Some(Vec::new());
    }
    // A full reply is smaller when very little of the encoded stream is reusable.
    if inserted.len() + 8 >= new.len() && !inserted.is_empty() {
        return None;
    }
    Some(vec![
        json!({ "start": prefix, "deleteCount": removed, "data": inserted }),
    ])
}
