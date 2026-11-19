use std::collections::HashMap;

type TokenId     = i64;
type ChainId     = i64;
type TokenDist   = HashMap<TokenId, i64>;
type TokenChain  = HashMap<TokenId, TokenDist>;
type TokenChains = HashMap<ChainId, TokenChain>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialToken {
    Start,
    End,
    NoItem,
    ZeroGram,
    WordListStart,
}

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
            TokenValue::Normal(bytes)         => bytes.as_slice(),
        }
    }
}

impl AsRef<[u8]> for TokenValue {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

fn main() {
    println!("Hello, world!");
}
