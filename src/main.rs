mod io;
mod math;
mod text;
mod tokens;
mod wordbag;

use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use zzz::ProgressBarIterExt as _;

const CTX_LEN: usize = 24;
const SAMPLE_SEQ: usize = 10000;
const PREFILL_CT: u8 = 0;
const NUM_SAMPLES: usize = 10;
const BACKOFF_ALPHA: f64 = 0.25;

#[cfg(feature = "chain_zero")]
const CHAINS_START: usize = 0;

#[cfg(not(feature = "chain_zero"))]
const CHAINS_START: usize = 1;

fn train_chain_ngram(
    chain: &mut tokens::TokenChain,
    bag: &wordbag::WordBag,
    n_gram: usize,
    next_tokens: [tokens::TokenId; u8::MAX as usize + 1],
    file_reader: &memmap2::Mmap,
    file_weight: u64,
) {
    let mut window_iterator = io::sliding_window_reader(file_reader, n_gram + 1);
    let mut dist: &mut tokens::TokenDist;
    let mut window: &[u8] = &[];

    let token_start = bag.get_id(tokens::SpecialToken::Start).unwrap();
    let token_zero_gram = bag.get_id(tokens::SpecialToken::ZeroGram).unwrap();

    // First window is special
    if n_gram != 0 {
        if let Some(i_window) = window_iterator.next() {
            window = i_window;

            let mut p_window = i_window.to_vec();

            // Token Start
            if let Some(&first) = p_window.first() {
                if text::is_valid_next_utf8_byte(first, &[]) {                    
                    // Using the weight for the start ones seem to generally produce poor resoults...
                    // TODO: Put a feature flag on this
                    // chain.entry(token_start).or_default().increment_weighted(next_tokens[first as usize], file_weight);
                    chain.entry(token_start).or_default().increment(next_tokens[first as usize]);
                }
            }

            // No, Jimmy, I don't care about files that start with 0x00 either
            for _ in 1..n_gram {
                // Splowly replace the window with
                // prefill from left to right
                // pushint the right out of frame
                p_window.rotate_right(1);
                p_window[0] = PREFILL_CT;

                if let Some((&next_t, prefix_t)) = p_window.split_last() {
                    if !prefix_t.iter().all(|&x| x == PREFILL_CT) && text::is_valid_next_utf8_byte(next_t, prefix_t) {
                        let prefix_t_t: tokens::TokenId;
                        if prefix_t.iter().all(|&x| x == 0) {
                            prefix_t_t = token_zero_gram;
                        } else {
                            prefix_t_t = bag.get_id(prefix_t).unwrap();
                        }
                        dist = chain.entry(prefix_t_t).or_default();
                        // Using the weight for the start ones seem to generally produce poor resoults...
                        // TODO: Put a feature flag on this
                        // dist.increment_weighted(next_tokens[next_t as usize], file_weight);
                        dist.increment(next_tokens[next_t as usize]);
                    }
                }
            }

            // Normal window proc
            if let Some((&next_t, prefix_t)) = window.split_last() {
                if text::is_valid_next_utf8_byte(next_t, prefix_t) {
                    dist = chain.entry(bag.get_id(prefix_t).unwrap()).or_default();
                    dist.increment_weighted(next_tokens[next_t as usize], file_weight);
                }
            }
        }
    }

    // Rest of the windows
    for i_window in window_iterator {
        window = i_window;
        if let Some((next_t, prefix_t)) = window.split_last() {
            let prefix_key = if n_gram == 0 { token_zero_gram } else { bag.get_id(prefix_t).unwrap() };
            dist = chain.entry(prefix_key).or_default();
            dist.increment_weighted(next_tokens[*next_t as usize], file_weight);
        }
    }

    // Last window is also special
    if n_gram != 0 && window.len() > 0 {
        dist = chain.entry(bag.get_id(&window[1..]).unwrap()).or_default();
        dist.increment_weighted(bag.get_id(tokens::SpecialToken::End).unwrap(), file_weight);
    }
}

fn train_chains_from_files(bag: &wordbag::WordBag, chains: &mut tokens::TokenChains, sources: &Vec<io::Source>) {
    let next_tokens: [tokens::TokenId; u8::MAX as usize + 1] = std::array::from_fn(|i| bag.get_id(tokens::TokenValue::Next(i as u8)).unwrap());
    let train_ngrams: Vec<usize> = math::build_active_orders();
    
    println!("train_chains_from_files:");
    for source in sources {
        for file in source.get_file_readers() {
            let chains_iter = chains.par_iter_mut().filter(|(chain_id, _)| {
                let n_gram = chain_id.0;

                let chain_range = file.chain_range.contains(&n_gram);
                let chain_train = train_ngrams.contains(&n_gram);
                let file_len = file.mmap.len() >= n_gram + 1;
                
                chain_range && chain_train && file_len
            });

            chains_iter.for_each(|(chain_id, chain)| {
                train_chain_ngram(chain, bag, chain_id.0, next_tokens, &file.mmap, file.weight);
            });
        }
    }
}

pub fn select_dist<'a>(
    dists: &[&'a tokens::TokenDist],
    zero_gram: &'a tokens::TokenDist,
    allowed: &[tokens::TokenId],
    alpha: f64,
    rng: &mut rand::rngs::SmallRng,
) -> &'a tokens::TokenDist {
    for &dist in dists {
        let (total, types) = dist.constrained_stats(allowed);
        if total == 0 {
            continue;
        }

        let backoff_mass = alpha * (types as f64);
        let p_backoff = backoff_mass / (total as f64 + backoff_mass);
        if rand::RngExt::random_range(rng, 0.0..1.0) >= p_backoff {
            return dist;
        }
    }

    zero_gram
}

fn sample_chains_backoff(bag: &wordbag::WordBag, chains: &tokens::TokenChains, backoff_alpha: f64) {
    let mut rng: rand::rngs::SmallRng = rand::make_rng();
    let mut prefix_t: [u8; CTX_LEN] = [PREFILL_CT; CTX_LEN];
    let mut output: Vec<u8> = Vec::with_capacity(SAMPLE_SEQ);

    let next_tokens: [tokens::TokenId; u8::MAX as usize + 1] = std::array::from_fn(|i| bag.get_id(tokens::TokenValue::Next(i as u8)).unwrap());
    let token_zero_gram = bag.get_id(tokens::SpecialToken::ZeroGram).unwrap();
    let token_start = bag.get_id(tokens::SpecialToken::Start).unwrap();
    let token_end = bag.get_id(tokens::SpecialToken::End).unwrap();
    let mut n_gram_iter = math::build_active_orders();
    n_gram_iter.reverse();

    println!("sample_chains:");
    'sample: for _ in (0..SAMPLE_SEQ).progress() {
        let mut allowed_next_ts: Vec<tokens::TokenId> =
            text::valid_next_utf8_bytes(&prefix_t).iter().cloned().flatten().map(|x| next_tokens[x as usize]).collect();
        allowed_next_ts.push(token_end);

        let dists: Vec<&tokens::TokenDist> = n_gram_iter
            .iter().filter_map(|&n_gram| {
                let this_prefix_t: tokens::TokenId;
                if n_gram == 0 {
                    this_prefix_t = token_zero_gram;
                } else if prefix_t.iter().all(|&x| x == PREFILL_CT) {
                    this_prefix_t = token_start;
                } else {
                    match bag.get_id(prefix_t.get((CTX_LEN - n_gram)..).unwrap()) {
                        Some(id) => this_prefix_t = id,
                        None => return None,
                    }
                };
                return chains.get(&tokens::ChainId(n_gram)).unwrap().get(&this_prefix_t);
            })
            .collect();

        #[cfg(not(feature = "chain_zero"))]
        let zero_gram_dist: &tokens::TokenDist = &tokens::TokenDist::flat(&allowed_next_ts);

        #[cfg(feature = "chain_zero")]
        let zero_gram_dist: &tokens::TokenDist = chains.get(&tokens::ChainId(0)).unwrap().get(&token_zero_gram).unwrap();

        let dist: &tokens::TokenDist = select_dist(&dists, zero_gram_dist, &allowed_next_ts, backoff_alpha, &mut rng);

        let sample: tokens::TokenId;
        #[cfg(feature = "constrained_sampler")]
        {
            sample = dist.sample_constrained(&mut rng, &allowed_next_ts).unwrap()
        };
        #[cfg(not(feature = "constrained_sampler"))]
        {
            sample = dist.sample_basic(&mut rng).unwrap()
        };

        match bag.i2s(sample) {
            Some(next_s) => {
                match next_s {
                    tokens::TokenValue::Next(next) => {
                        output.push(*next);
                        // The order and function are fucking correct, don't touch it
                        prefix_t[0] = *next;
                        prefix_t.rotate_left(1);
                    }
                    tokens::TokenValue::Special(special_t) => match special_t {
                        tokens::SpecialToken::End => break 'sample,
                        tokens::SpecialToken::Start => unreachable!(),
                        tokens::SpecialToken::ZeroGram => unreachable!(),
                        tokens::SpecialToken::WordListStart => unreachable!(),
                    },
                    tokens::TokenValue::Normal(_) => unreachable!(),
                }
            }
            None => {
                println!("Fuck.");
                dbg!(sample);
                dbg!(prefix_t);
                dbg!(text::decode_chopped_utf8(&prefix_t));
                panic!("WOOPS: We shouldn't have got here!!! #2")
            }
        };
    }

    println!();
    println!("SAMPLED:\n---\n\n{:}\n\n----\n", text::decode_chopped_utf8(&output));
    // println!("SAMPLED RAW:\t\t{:?}", &output);
    println!("PREFIX BUFFER STATE: {:?}", &prefix_t);
    println!("PREFIX BUFFER STATE (READABLE): {:?}", text::decode_chopped_utf8(&prefix_t));
    println!();
}

fn main() {
    let test_files: Vec<io::Source> = vec![
        io::Source::from_glob(1, CHAINS_START..=(CTX_LEN - 04), r"target/release/lib-ngramthingamajig-rs"),
        // io::Source::from_glob(0, CHAINS_START..=(CTX_LEN - 04), r"/home/subject/bin/*"),
        io::Source::from_glob(1, CHAINS_START..=(CTX_LEN - 04), r"src/*.rs"),
    ];

    // init
    let mut chains: tokens::TokenChains = tokens::TokenChains::new();

    #[cfg(feature = "chain_zero")]
    chains.insert(tokens::ChainId(0), std::collections::HashMap::with_capacity_and_hasher(1, ahash::RandomState::new()));

    chains.insert(tokens::ChainId(1), std::collections::HashMap::with_capacity_and_hasher(u8::MAX as usize, ahash::RandomState::new()));

    for n_gram in 2..=CTX_LEN {
        chains.insert(tokens::ChainId(n_gram), std::collections::HashMap::with_hasher(ahash::RandomState::new()));
    }

    let mut bag = wordbag::WordBag::with_capacity((u8::MAX as usize) * 5);
    bag.setup_special_tokens();
    bag.train_from_files(&test_files);

    train_chains_from_files(&bag, &mut chains, &test_files);

    for _ in 0..NUM_SAMPLES {
        println!("\n\nSAMPLE WITH BACKOFF_ALPHA = {:}\n", BACKOFF_ALPHA);
        sample_chains_backoff(&bag, &chains, BACKOFF_ALPHA);
        println!("SAMPLE WITH BACKOFF_ALPHA = {:}", BACKOFF_ALPHA * 2.0);
        sample_chains_backoff(&bag, &chains, BACKOFF_ALPHA * 2.0);
        println!("SAMPLE WITH BACKOFF_ALPHA = {:}", BACKOFF_ALPHA * 10.0);
        sample_chains_backoff(&bag, &chains, BACKOFF_ALPHA * 10.0);
    }
}
