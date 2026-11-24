use std::hash::{BuildHasher, Hash, Hasher};

use ahash::RandomState;
use indexmap::{Equivalent, IndexSet};

use crate::io;
use crate::text;
use crate::tokens::{SpecialToken, TokenId, TokenValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenRef<'a> {
    Special(SpecialToken),
    Normal(&'a [u8]),
    Next(u8),
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
                byte.hash(state);
            }
        }
    }
}

impl Equivalent<TokenValue> for TokenRef<'_> {
    fn equivalent(&self, key: &TokenValue) -> bool {
        match (self, key) {
            (TokenRef::Special(a), TokenValue::Special(b)) => a == b,
            (TokenRef::Normal(a), TokenValue::Normal(b)) => *a == b.as_slice(),
            (TokenRef::Next(a), TokenValue::Next(b)) => a == b,
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
            TokenValue::Next(byte) => TokenRef::Next(*byte),
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
        TokenRef::Next(**self)
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
            TokenRef::Next(byte) => TokenValue::Next(byte),
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
        for byte in u8::MIN..=u8::MAX {
            self.s2i(TokenValue::Next(byte));
        }
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

    pub fn train_from_files(&mut self, sources: &Vec<io::Source>, ctx_len: usize, prefill_ct: u8) {
        println!("train_wordbag_from_files:");

        #[cfg_attr(not(feature = "experiment-n_gram-skips"), allow(unused_labels))]
        'n_gram_iter: for n_gram in 1..=ctx_len {
            #[cfg(feature = "experiment-n_gram-skips")]
            if !n_gram.is_power_of_two() && 1 < n_gram && n_gram < ctx_len {
                continue 'n_gram_iter;
            };

            'files_iter: for files in sources {
                if !files.chain_range.contains(&n_gram) {
                    continue 'files_iter;
                };
                'file_iter: for file in files.get_file_readers() {
                    if file.mmap.len() < n_gram + 1 {
                        continue 'file_iter;
                    };
                    let mut window_iterator = io::sliding_window_reader(&file.mmap, n_gram + 1);
                    let mut window: &[u8] = &[];

                    // First window is special
                    if n_gram != 0 {
                        if let Some(i_window) = window_iterator.next() {
                            window = i_window;

                            let mut p_window = i_window.to_vec();

                            while p_window.iter().any(|&x| x != prefill_ct) {
                                // Splowly replace the window with
                                // prefill from left to right
                                // pushint the right out of frame
                                p_window.rotate_right(1);
                                p_window[0] = prefill_ct;

                                if let Some((&next_t, prefix_t)) = p_window.split_last() {
                                    if text::is_valid_next_utf8_byte(next_t, prefix_t) {
                                        self.s2i(prefix_t);
                                    }
                                }
                            }

                            // Normal window proc
                            if let Some((&next_t, prefix_t)) = window.split_last() {
                                if text::is_valid_next_utf8_byte(next_t, prefix_t) {
                                    self.s2i(prefix_t);
                                }
                            }
                        }
                    }

                    // Rest of the windows
                    for i_window in window_iterator {
                        window = i_window;
                        if let Some((_, prefix_t)) = window.split_last() {
                            self.s2i(prefix_t);
                        }
                    }

                    // Last window is also special
                    if window.len() > 0 {
                        self.s2i(&window[1..]);
                    }
                }
            }
        }
    }
}
