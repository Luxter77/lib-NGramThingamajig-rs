use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

use ahash::RandomState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TokenId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ChainId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TokenCount(pub u64);

#[derive(Debug, Clone, Default)]
pub enum TokenDist {
    #[default]
    Empty,
    Single(TokenId, TokenCount),
    Many(Box<HashMap<TokenId, TokenCount, RandomState>>),
}

impl TokenDist {
    pub fn add(&mut self, token: TokenId, amount: u64) {
        match self {
            TokenDist::Empty => {
                *self = TokenDist::Single(token, TokenCount(amount));
            }
            TokenDist::Single(existing_id, count) if *existing_id == token => {
                count.0 += amount;
            }
            TokenDist::Single(existing_id, count) => {
                let mut map = HashMap::with_capacity_and_hasher(2, RandomState::new());
                map.insert(*existing_id, *count);
                map.insert(token, TokenCount(amount));
                *self = TokenDist::Many(Box::new(map));
            }
            TokenDist::Many(map) => {
                map.entry(token).or_default().0 += amount;
            }
        }
    }

    pub fn flat(tokens: &[TokenId]) -> Self {
        let mut this = TokenDist::Many(Box::new(HashMap::with_capacity_and_hasher(tokens.len(), RandomState::new())));

        for &token in tokens {
            this.add(token, 1);
        }

        return this;
    }

    pub fn increment(&mut self, token: TokenId) {
        self.add(token, 1);
    }

    pub fn increment_weighted(&mut self, token: TokenId, weight: u64) {
        self.add(token, weight);
    }

    pub fn with_capacity(capacity: usize) -> Self {
        if capacity <= 1 { TokenDist::Empty } else { TokenDist::Many(Box::new(HashMap::with_capacity_and_hasher(capacity, RandomState::new()))) }
    }

    #[cfg(not(feature = "constrained_sampler"))]
    pub fn sample_basic(&self, rng: &mut rand::rngs::SmallRng) -> Option<TokenId> {
        // The lion does not concern himself with the opinion of the UTF-8 state machine
        match &self {
            TokenDist::Empty => return None,
            TokenDist::Single(id, _) => return Some(*id),
            TokenDist::Many(map) => {
                let total = map.values().map(|count| count.0).sum();
                if total == 0 {
                    return None;
                };
                let mut pick = rand::RngExt::random_range(rng, 0..total);
                for (id, count) in map.iter() {
                    if pick < count.0 {
                        return Some(*id);
                    }
                    pick -= count.0;
                }
                return None;
            }
        }
    }

    #[cfg(feature = "constrained_sampler")]
    pub fn sample_constrained(&self, rng: &mut rand::rngs::SmallRng, allowed: &[TokenId]) -> Option<TokenId> {
        match &self {
            TokenDist::Empty => return None,
            TokenDist::Single(id, count) => {
                if count.0 > 0 && allowed.contains(id) {
                    Some(*id)
                } else {
                    None
                }
            }
            TokenDist::Many(map) => {
                let map: ahash::HashMap<&TokenId, &TokenCount> = map.iter().filter(|(id, _)| allowed.contains(*id)).collect();
                let total = map.values().map(|count| count.0).sum();
                if total == 0 {
                    return None;
                };
                let mut pick = rand::RngExt::random_range(rng, 0..total);
                for (&id, &count) in map.iter() {
                    if pick < count.0 {
                        return Some(*id);
                    }
                    pick -= count.0;
                }
                return None;
            }
        }
    }

    pub fn constrained_stats(&self, allowed: &[TokenId]) -> (u64, u64) {
        match self {
            TokenDist::Empty => (0, 0),
            TokenDist::Single(id, count) => {
                if count.0 > 0 && allowed.contains(id) {
                    (count.0, 1)
                } else {
                    (0, 0)
                }
            }
            TokenDist::Many(map) => {
                let mut total: u64 = 0;
                let mut types: u64 = 0;
                for (id, count) in map.iter() {
                    if count.0 > 0 && allowed.contains(id) {
                        total += count.0;
                        types += 1;
                    }
                }
                (total, types)
            }
        }
    }
}

pub type TokenChain = HashMap<TokenId, TokenDist, RandomState>;
pub type TokenChains = BTreeMap<ChainId, TokenChain>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialToken {
    Start,
    End,
    ZeroGram,
    WordListStart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenValue {
    Special(SpecialToken),
    Normal(Vec<u8>),
    Next(u8),
}

impl Hash for TokenValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            TokenValue::Special(special) => {
                0u8.hash(state);
                special.hash(state);
            }
            TokenValue::Normal(bytes) => {
                1u8.hash(state);
                bytes.as_slice().hash(state);
            }
            TokenValue::Next(byte) => {
                2u8.hash(state);
                byte.hash(state);
            }
        }
    }
}

impl SpecialToken {
    pub const fn as_bytes(&self) -> &'static [u8] {
        match self {
            SpecialToken::Start => b"__TOKEN__START__",
            SpecialToken::End => b"___TOKEN__END___",
            SpecialToken::ZeroGram => b"__TOKEN_0_GRAM__",
            SpecialToken::WordListStart => b"_TOKEN_WL_START_",
        }
    }
}

impl TokenValue {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            TokenValue::Special(special) => special.as_bytes(),
            TokenValue::Normal(bytes) => bytes.as_slice(),
            TokenValue::Next(byte) => std::slice::from_ref(byte),
        }
    }
}

impl AsRef<[u8]> for TokenValue {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
