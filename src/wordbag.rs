use std::hash::{BuildHasher, Hash, Hasher};

use ahash::RandomState;
use indexmap::{Equivalent, IndexSet};

use crate::io::{get_file_readers, sliding_window_reader};
use crate::tokens::{SpecialToken, TokenId, TokenValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenRef<'a> {
    Special(SpecialToken),
    Normal(&'a [u8]),
    Next(&'a u8),
}

impl Hash for TokenRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            TokenRef::Special(special) => {
                0u8.hash(state);
                special.hash(state);
            }
            TokenRef::Normal(bytes) => {
                1u8.hash(state);
                bytes.hash(state);
            }
            TokenRef::Next(byte) => {
                2u8.hash(state);
                std::slice::from_ref(byte).hash(state);
            }
        }
    }
}

impl Equivalent<TokenValue> for TokenRef<'_> {
    fn equivalent(&self, key: &TokenValue) -> bool {
        match (self, key) {
            (TokenRef::Special(a), TokenValue::Special(b)) => a == b,
            (TokenRef::Normal(a), TokenValue::Normal(b)) => *a == b.as_slice(),
            (TokenRef::Next(a), TokenValue::Next(b)) => *a == b,
            _ => false,
        }
    }
}

pub trait TokenInput {
    fn as_token_ref(&self) -> TokenRef<'_>;
    fn into_token_value(self) -> TokenValue;
}

impl TokenInput for TokenValue {
    fn as_token_ref(&self) -> TokenRef<'_> {
        match self {
            TokenValue::Special(special) => TokenRef::Special(*special),
            TokenValue::Normal(bytes) => TokenRef::Normal(bytes.as_slice()),
            TokenValue::Next(byte) => TokenRef::Next(byte),
        }
    }

    fn into_token_value(self) -> TokenValue {
        self
    }
}

impl TokenInput for &TokenValue {
    fn as_token_ref(&self) -> TokenRef<'_> {
        <TokenValue as TokenInput>::as_token_ref(*self)
    }

    fn into_token_value(self) -> TokenValue {
        TokenValue::clone(self)
    }
}

impl TokenInput for SpecialToken {
    fn as_token_ref(&self) -> TokenRef<'_> {
        TokenRef::Special(*self)
    }

    fn into_token_value(self) -> TokenValue {
        TokenValue::Special(self)
    }
}

impl TokenInput for &[u8] {
    fn as_token_ref(&self) -> TokenRef<'_> {
        TokenRef::Normal(*self)
    }

    fn into_token_value(self) -> TokenValue {
        TokenValue::Normal(self.to_vec())
    }
}

impl TokenInput for &u8 {
    fn as_token_ref(&self) -> TokenRef<'_> {
        TokenRef::Next(*self)
    }

    fn into_token_value(self) -> TokenValue {
        TokenValue::Next(*self)
    }
}

impl<const N: usize> TokenInput for &[u8; N] {
    fn as_token_ref(&self) -> TokenRef<'_> {
        TokenRef::Normal(self.as_slice())
    }

    fn into_token_value(self) -> TokenValue {
        TokenValue::Normal(self.to_vec())
    }
}

impl TokenInput for TokenRef<'_> {
    fn as_token_ref(&self) -> TokenRef<'_> {
        *self
    }

    fn into_token_value(self) -> TokenValue {
        match self {
            TokenRef::Special(special) => TokenValue::Special(special),
            TokenRef::Normal(bytes) => TokenValue::Normal(bytes.to_vec()),
            TokenRef::Next(byte) => TokenValue::Next(*byte),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WordBag<S = RandomState> {
    tokens: IndexSet<TokenValue, S>,
}

impl Default for WordBag<RandomState> {
    fn default() -> Self {
        Self::new()
    }
}

impl WordBag<RandomState> {
    pub fn new() -> Self {
        Self::with_capacity_and_hasher(0, RandomState::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, RandomState::new())
    }

    pub fn setup_special_tokens(&mut self) {
        self.s2i(SpecialToken::Start);
        self.s2i(SpecialToken::End);
        self.s2i(SpecialToken::ZeroGram);
        self.s2i(SpecialToken::WordListStart);
    }
}

impl<S: BuildHasher> WordBag<S> {
    pub fn with_hasher(hasher: S) -> Self {
        Self::with_capacity_and_hasher(0, hasher)
    }

    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        Self { tokens: IndexSet::with_capacity_and_hasher(capacity, hasher) }
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn i2s(&self, p_token: TokenId) -> Option<&TokenValue> {
        let idx = usize::try_from(p_token.0).ok()?;
        self.tokens.get_index(idx)
    }

    pub fn get_id<T: TokenInput>(&self, s_token: T) -> Option<TokenId> {
        self.tokens.get_index_of(&s_token.as_token_ref()).map(|idx| TokenId(idx as u64))
    }

    pub fn s2i<T: TokenInput>(&mut self, s_token: T) -> TokenId {
        if let Some(id) = self.get_id(s_token.as_token_ref()) {
            return id;
        }

        let (idx, _) = self.tokens.insert_full(s_token.into_token_value());
        TokenId(idx as u64)
    }

    pub fn keys(&self) -> impl ExactSizeIterator<Item = TokenId> {
        (0..self.tokens.len()).map(|idx| TokenId(idx as u64))
    }

    pub fn values(&self) -> impl ExactSizeIterator<Item = &TokenValue> {
        self.tokens.iter()
    }

    pub fn train_from_files(&mut self, files: &Vec<std::path::PathBuf>, ctx_len: usize) {
        println!("train_wordbag_from_files:");

        'n_gram_iter: for n_gram in 1..=ctx_len {
            #[cfg(feature = "experiment-n_gram-skips")]
            if !n_gram.is_power_of_two() && 1 < n_gram && n_gram < ctx_len {
                continue 'n_gram_iter;
            };
            for file_reader in get_file_readers(files) {
                match file_reader {
                    Ok(file_reader) => {
                        if file_reader.len() < n_gram + 1 {
                            continue 'n_gram_iter;
                        };
                        for window in sliding_window_reader(&file_reader, n_gram) {
                            self.s2i(window);
                        }
                    }
                    Err(_) => unreachable!(),
                }
            }
        }
    }
}
