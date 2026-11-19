use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)] pub struct TokenId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)] pub struct ChainId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)] pub struct TokenCount(pub u64);

#[derive(Debug, Clone, Default)]
pub enum TokenDist {
    #[default]
    Empty,
    Single(TokenId, TokenCount),
    Many(Box<HashMap<TokenId, TokenCount>>),
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
                let mut map = HashMap::with_capacity(2);
                map.insert(*existing_id, *count);
                map.insert(token, TokenCount(amount));
                *self = TokenDist::Many(Box::new(map));
            }
            TokenDist::Many(map) => {
                map.entry(token).or_default().0 += amount;
            }
        }
    }

    pub fn increment(&mut self, token: TokenId) {
        self.add(token, 1);
    }
}

type TokenChain  = HashMap<TokenId,  TokenDist>;
type TokenChains = BTreeMap<ChainId, TokenChain>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] pub enum SpecialToken { Start, End, NoItem, ZeroGram, WordListStart }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenValue {
    Special(SpecialToken),
    Normal(Vec<u8>),
}

impl SpecialToken {
    pub const fn as_bytes(&self) -> &'static [u8] {
        match self {
            SpecialToken::Start         => b"__TOKEN__START__",
            SpecialToken::End           => b"___TOKEN__END___",
            SpecialToken::ZeroGram      => b"__TOKEN_0_GRAM__",
            SpecialToken::NoItem        => b"__TOKEN_NOITEM__",
            SpecialToken::WordListStart => b"_TOKEN_WL_START_",
        }
    }
}

impl TokenValue {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            TokenValue::Special(special) => special.as_bytes(),
            TokenValue::Normal(bytes) => bytes.as_slice(),
        }
    }
}

impl AsRef<[u8]> for TokenValue {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}