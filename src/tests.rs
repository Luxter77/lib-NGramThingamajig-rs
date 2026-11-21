use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

use crate::tokens::{
    ChainId, SpecialToken, TokenChain, TokenChains, TokenCount, TokenDist, TokenId, TokenValue,
};
use crate::wordbag::{TokenInput, TokenRef, WordBag};

const PARAGRAPH: &str = "the quick brown fox jumps over the lazy dog. \
the dog barks and the fox runs. \
the lazy dog sleeps while the quick fox jumps.";

const WINDOW: usize = 3;

pub fn run() {
    println!(
        "Size of Message: {} bytes",
        std::mem::size_of::<TokenValue>()
    );
    // You can also check it at compile time:
    const SIZE: usize = std::mem::size_of::<TokenValue>();
    assert_eq!(SIZE, 24); // Rust uses niche optimization on String's pointer!

    let mut wordbag = WordBag::with_capacity(64);

    let start = wordbag.s2i(SpecialToken::Start);
    let end = wordbag.s2i(SpecialToken::End);
    let no_item = wordbag.s2i(SpecialToken::NoItem);
    let zero_gram = wordbag.s2i(SpecialToken::ZeroGram);
    let word_list_start = wordbag.s2i(SpecialToken::WordListStart);

    let windows: Vec<&[u8]> = PARAGRAPH.as_bytes().windows(WINDOW).collect();
    let ids: Vec<TokenId> = windows.iter().map(|window| wordbag.s2i(*window)).collect();

    println!("paragraph: {} bytes", PARAGRAPH.len());
    println!("window: {WINDOW} bytes, {} windows", windows.len());
    println!("vocabulary: {} tokens", wordbag.len());
    println!(
        "start={start:?} end={end:?} no_item={no_item:?} zero_gram={zero_gram:?} word_list_start={word_list_start:?}"
    );

    assert_eq!(ids.len(), PARAGRAPH.len() - WINDOW + 1);
    assert!(!wordbag.is_empty());

    let mut chain: TokenChain = TokenChain::new();
    for pair in ids.windows(2) {
        chain
            .entry(pair[0])
            .or_insert_with(|| TokenDist::with_capacity(2))
            .increment(pair[1]);
    }

    let mut chains = TokenChains::new();
    for (position, pair) in ids.windows(2).enumerate() {
        let mut step: TokenChain = TokenChain::new();
        step.entry(pair[0]).or_default().increment(pair[1]);
        chains.insert(ChainId(position), step);
    }

    println!("chain: {} distinct windows", chain.len());
    println!("chains: {} steps", chains.len());

    let mut ranked: Vec<(&TokenId, &TokenDist)> = chain.iter().collect();
    ranked.sort_by_key(|(token, dist)| (std::cmp::Reverse(total(dist)), token.0));

    for (token, dist) in ranked.iter().take(8) {
        println!(
            "{:?} -> {}",
            text_of(**token, &wordbag),
            render_dist(dist, &wordbag)
        );
    }

    for (chain_id, step) in chains.iter().take(3) {
        for (token, dist) in step {
            println!(
                "{chain_id:?} {:?} -> {}",
                text_of(*token, &wordbag),
                render_dist(dist, &wordbag)
            );
        }
    }

    let first_window = windows[0];
    let first_id = ids[0];
    assert_eq!(wordbag.get_id(first_window), Some(first_id));
    assert_eq!(wordbag.get_id(b"missing".as_slice()), None);

    let token_ref = TokenRef::Normal(first_window);
    assert_eq!(wordbag.get_id(token_ref), Some(first_id));
    assert_eq!(
        token_ref.into_token_value(),
        TokenValue::Normal(first_window.to_vec())
    );
    assert_eq!(
        TokenValue::Special(SpecialToken::End).as_token_ref(),
        TokenRef::Special(SpecialToken::End)
    );

    let ids_from_keys: Vec<TokenId> = wordbag.keys().collect();
    assert_eq!(ids_from_keys.len(), wordbag.len());

    for id in ids_from_keys.iter().take(10) {
        let value = wordbag.i2s(*id).expect("id must resolve");
        println!(
            "{id:?} -> {:?} ({})",
            value,
            String::from_utf8_lossy(value.as_bytes())
        );
    }

    for value in wordbag.values().take(10) {
        println!("value: {:?}", value.as_bytes());
    }

    let count = TokenCount(4);
    println!("count={count:?}");

    let hasher_bag: WordBag = WordBag::with_hasher(ahash::RandomState::new());
    assert!(hasher_bag.is_empty());
    assert_eq!(hasher_bag.len(), 0);
    assert_eq!(hasher_bag.get_id(SpecialToken::Start), None);

    let mut rng = SmallRng::seed_from_u64(42);
    let seed = ids[0];
    let generated = generate(&chain, seed, 40, &mut rng);

    println!("seed: {:?}", text_of(seed, &wordbag));
    println!("generated: {:?}", render_text(&generated, &wordbag));
    println!("generated ids: {} tokens", generated.len());

    assert_eq!(generated.first(), Some(&seed));
    assert!(generated.len() <= 41);
    for pair in generated.windows(2) {
        let dist = chain
            .get(&pair[0])
            .expect("generated token must be in the chain");
        assert!(
            count_of(dist, pair[1]) > 0,
            "transition must exist in the chain"
        );
    }
}

fn generate(chain: &TokenChain, seed: TokenId, steps: usize, rng: &mut SmallRng) -> Vec<TokenId> {
    let mut sequence = vec![seed];
    let mut current = seed;

    for _ in 0..steps {
        let Some(dist) = chain.get(&current) else {
            break;
        };
        let Some(next) = sample(dist, rng) else {
            break;
        };
        sequence.push(next);
        current = next;
    }

    sequence
}

// this shouldnt actually be the one used but while developing its te best one
pub fn sample(dist: &TokenDist, rng: &mut SmallRng) -> Option<TokenId> {
    let total = total(dist);
    if total == 0 {
        return None;
    }

    let mut pick = rng.random_range(0..total);
    match dist {
        TokenDist::Empty => None,
        TokenDist::Single(id, _) => Some(*id),
        TokenDist::Many(map) => {
            for (id, count) in map.iter() {
                if pick < count.0 {
                    return Some(*id);
                }
                pick -= count.0;
            }
            None
        }
    }
}

fn render_text(ids: &[TokenId], wordbag: &WordBag) -> String {
    let mut text = String::new();
    for (position, id) in ids.iter().enumerate() {
        let Some(value) = wordbag.i2s(*id) else {
            continue;
        };
        let bytes = value.as_bytes();
        if position == 0 {
            text.push_str(&String::from_utf8_lossy(bytes));
        } else if let Some(last) = bytes.last() {
            text.push(*last as char);
        }
    }
    text
}

fn text_of(id: TokenId, wordbag: &WordBag) -> String {
    wordbag
        .i2s(id)
        .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
        .unwrap_or_else(|| format!("{id:?}"))
}

fn render_dist(dist: &TokenDist, wordbag: &WordBag) -> String {
    match dist {
        TokenDist::Empty => String::from("(empty)"),
        TokenDist::Single(id, count) => format!("{:?} x{}", text_of(*id, wordbag), count.0),
        TokenDist::Many(map) => {
            let mut parts: Vec<String> = map
                .iter()
                .map(|(id, count)| format!("{:?} x{}", text_of(*id, wordbag), count.0))
                .collect();
            parts.sort();
            parts.join(", ")
        }
    }
}

fn total(dist: &TokenDist) -> u64 {
    match dist {
        TokenDist::Empty => 0,
        TokenDist::Single(_, count) => count.0,
        TokenDist::Many(map) => map.values().map(|count| count.0).sum(),
    }
}

fn count_of(dist: &TokenDist, token: TokenId) -> u64 {
    match dist {
        TokenDist::Empty => 0,
        TokenDist::Single(id, count) => u64::from(*id == token) * count.0,
        TokenDist::Many(map) => map.get(&token).map_or(0, |count| count.0),
    }
}

#[cfg(test)]
fn build_chain(paragraph: &str, window: usize) -> (WordBag, TokenChain) {
    let mut wordbag = WordBag::new();
    let ids: Vec<TokenId> = paragraph
        .as_bytes()
        .windows(window)
        .map(|window| wordbag.s2i(window))
        .collect();

    let mut chain: TokenChain = TokenChain::new();
    for pair in ids.windows(2) {
        chain.entry(pair[0]).or_default().increment(pair[1]);
    }

    (wordbag, chain)
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn token_value_has_expected_size() {
        assert_eq!(std::mem::size_of::<TokenValue>(), 24);
    }

    #[test]
    fn special_tokens_have_distinct_bytes() {
        let tokens = [
            SpecialToken::Start,
            SpecialToken::End,
            SpecialToken::NoItem,
            SpecialToken::ZeroGram,
            SpecialToken::WordListStart,
        ];

        let mut seen = std::collections::HashSet::new();
        for token in tokens {
            assert!(
                seen.insert(token.as_bytes()),
                "duplicate bytes for {token:?}"
            );
        }
        assert_eq!(seen.len(), tokens.len());
    }

    #[test]
    fn s2i_is_idempotent() {
        let mut wordbag = WordBag::new();

        let first = wordbag.s2i(b"alpha".as_slice());
        let second = wordbag.s2i(b"alpha".as_slice());
        let third = wordbag.s2i(TokenValue::Normal(b"alpha".to_vec()));

        assert_eq!(first, second);
        assert_eq!(first, third);
        assert_eq!(wordbag.len(), 1);
    }

    #[test]
    fn special_and_normal_tokens_are_distinct() {
        let mut wordbag = WordBag::new();

        let special = wordbag.s2i(SpecialToken::Start);
        let normal = wordbag.s2i(SpecialToken::Start.as_bytes());

        assert_ne!(special, normal);
        assert_eq!(
            wordbag.get_id(TokenRef::Special(SpecialToken::Start)),
            Some(special)
        );
        assert_eq!(
            wordbag.get_id(TokenRef::Normal(SpecialToken::Start.as_bytes())),
            Some(normal)
        );
    }

    #[test]
    fn i2s_roundtrips_every_id() {
        let mut wordbag = WordBag::new();

        let ids = [
            wordbag.s2i(SpecialToken::Start),
            wordbag.s2i(b"alpha".as_slice()),
            wordbag.s2i(b"beta".as_slice()),
        ];

        for id in ids {
            assert!(wordbag.i2s(id).is_some());
        }
        assert_eq!(wordbag.i2s(TokenId(u64::MAX)), None);
    }

    #[test]
    fn get_id_returns_none_for_unknown() {
        let mut wordbag = WordBag::new();
        wordbag.s2i(b"alpha".as_slice());

        assert_eq!(wordbag.get_id(b"missing".as_slice()), None);
        assert_eq!(wordbag.get_id(SpecialToken::End), None);
    }

    #[test]
    fn keys_and_values_align() {
        let mut wordbag = WordBag::new();
        wordbag.s2i(SpecialToken::Start);
        wordbag.s2i(b"alpha".as_slice());
        wordbag.s2i(b"beta".as_slice());

        let keys: Vec<TokenId> = wordbag.keys().collect();
        let values: Vec<&TokenValue> = wordbag.values().collect();

        assert_eq!(keys.len(), values.len());
        for (id, value) in keys.iter().zip(values) {
            assert_eq!(wordbag.i2s(*id), Some(value));
        }
    }

    #[test]
    fn token_input_conversions_preserve_value() {
        let bytes: &[u8] = b"abc";

        assert_eq!(
            bytes.into_token_value(),
            TokenValue::Normal(b"abc".to_vec())
        );
        assert_eq!(
            b"abc".into_token_value(),
            TokenValue::Normal(b"abc".to_vec())
        );
        assert_eq!(
            SpecialToken::End.into_token_value(),
            TokenValue::Special(SpecialToken::End)
        );

        let value = TokenValue::Normal(b"abc".to_vec());
        assert_eq!((&value).into_token_value(), value);
        assert_eq!(TokenRef::Normal(bytes).into_token_value(), value);
    }

    #[test]
    fn token_dist_promotes_single_to_many() {
        let mut dist = TokenDist::default();
        assert!(matches!(dist, TokenDist::Empty));

        dist.increment(TokenId(1));
        dist.add(TokenId(1), 2);

        match &dist {
            TokenDist::Single(id, count) => {
                assert_eq!(*id, TokenId(1));
                assert_eq!(count.0, 3);
            }
            other => panic!("expected Single, got {other:?}"),
        }

        dist.increment(TokenId(2));

        match &dist {
            TokenDist::Many(map) => {
                assert_eq!(map.get(&TokenId(1)).map(|count| count.0), Some(3));
                assert_eq!(map.get(&TokenId(2)).map(|count| count.0), Some(1));
            }
            other => panic!("expected Many, got {other:?}"),
        }
    }

    #[test]
    fn with_capacity_selects_variant_by_capacity() {
        assert!(matches!(TokenDist::with_capacity(0), TokenDist::Empty));
        assert!(matches!(TokenDist::with_capacity(1), TokenDist::Empty));

        match TokenDist::with_capacity(4) {
            TokenDist::Many(map) => {
                assert!(map.is_empty());
                assert!(map.capacity() >= 4);
            }
            other => panic!("expected Many, got {other:?}"),
        }
    }

    #[test]
    fn with_capacity_accepts_additions() {
        let mut dist = TokenDist::with_capacity(8);
        dist.increment(TokenId(1));
        dist.add(TokenId(2), 3);

        match &dist {
            TokenDist::Many(map) => {
                assert_eq!(map.get(&TokenId(1)).map(|count| count.0), Some(1));
                assert_eq!(map.get(&TokenId(2)).map(|count| count.0), Some(3));
            }
            other => panic!("expected Many, got {other:?}"),
        }
    }

    #[test]
    fn sliding_window_chain_records_every_transition() {
        let mut wordbag = WordBag::new();
        let ids: Vec<TokenId> = PARAGRAPH
            .as_bytes()
            .windows(WINDOW)
            .map(|window| wordbag.s2i(window))
            .collect();

        let mut chain: TokenChain = TokenChain::new();
        for pair in ids.windows(2) {
            chain.entry(pair[0]).or_default().increment(pair[1]);
        }

        let transitions: u64 = chain.values().map(total).sum();
        assert_eq!(transitions as usize, ids.len() - 1);

        for pair in ids.windows(2) {
            let dist = chain
                .get(&pair[0])
                .expect("source window must be in the chain");
            assert!(count_of(dist, pair[1]) > 0);
        }
    }

    #[test]
    fn sample_returns_only_known_transitions() {
        let (_, chain) = build_chain(PARAGRAPH, WINDOW);

        let mut rng = SmallRng::seed_from_u64(7);
        for dist in chain.values() {
            for _ in 0..20 {
                let picked = sample(dist, &mut rng).expect("non-empty distribution must sample");
                assert!(
                    count_of(dist, picked) > 0,
                    "sample must come from the distribution"
                );
            }
        }
    }

    #[test]
    fn sample_returns_none_for_empty_distribution() {
        let mut rng = SmallRng::seed_from_u64(7);
        assert_eq!(sample(&TokenDist::Empty, &mut rng), None);
    }

    #[test]
    fn generate_follows_chain_transitions() {
        let (_, chain) = build_chain(PARAGRAPH, WINDOW);

        let mut rng = SmallRng::seed_from_u64(42);
        let seed = *chain.keys().next().expect("chain must not be empty");
        let generated = generate(&chain, seed, 40, &mut rng);

        assert_eq!(generated.first(), Some(&seed));
        assert!(generated.len() > 1);
        for pair in generated.windows(2) {
            let dist = chain
                .get(&pair[0])
                .expect("generated token must be in the chain");
            assert!(count_of(dist, pair[1]) > 0);
        }
    }

    #[test]
    fn low_order_chain_keeps_every_transition() {
        for window in 1..=2 {
            let (_, chain) = build_chain(PARAGRAPH, window);
            let mut rng = SmallRng::seed_from_u64(11);

            for (source, dist) in &chain {
                for _ in 0..20 {
                    let picked =
                        sample(dist, &mut rng).expect("non-empty distribution must sample");
                    assert!(
                        count_of(dist, picked) > 0,
                        "order {window} source {source:?} sampled {picked:?} outside its distribution"
                    );
                }
            }
        }
    }

    #[test]
    fn high_order_chain_samples_stay_in_vocabulary() {
        let (wordbag, chain) = build_chain(PARAGRAPH, WINDOW);
        let mut rng = SmallRng::seed_from_u64(13);

        for dist in chain.values() {
            for _ in 0..20 {
                let picked = sample(dist, &mut rng).expect("non-empty distribution must sample");
                assert!(
                    wordbag.i2s(picked).is_some(),
                    "order {WINDOW} sampled {picked:?} outside the vocabulary"
                );
            }
        }
    }
}
