mod io;
mod tests;
mod tokens;
mod wordbag;

use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use zzz::ProgressBarIterExt as _;

const CTX_LEN: usize = 15;
const SAMPLE_SEQ: usize = 2000;
const PREFILL_CT: u8 = 0;
const NUM_SAMPLES: usize = 10;

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
            chain.entry(token_start).or_default().increment(next_tokens[*p_window.first().unwrap() as usize]);

            while p_window.iter().any(|&x| x != PREFILL_CT) {
                // Splowly replace the window with
                // prefill from left to right
                // pushint the right out of frame
                p_window.rotate_right(1);
                p_window[0] = PREFILL_CT;

                if let Some((next_t, prefix_t)) = p_window.split_last() {
                    dist = chain.entry(bag.get_id(prefix_t).unwrap()).or_default();
                    dist.increment(next_tokens[*next_t as usize]);
                }
            }

            // Normal window proc
            if let Some((next_t, prefix_t)) = window.split_last() {
                dist = chain.entry(bag.get_id(prefix_t).unwrap()).or_default();
                dist.increment(next_tokens[*next_t as usize]);
            }
        }
    }

    // Rest of the windows
    for i_window in window_iterator {
        window = i_window;
        if let Some((next_t, prefix_t)) = window.split_last() {
            let prefix_key = if n_gram == 0 { token_zero_gram } else { bag.get_id(prefix_t).unwrap() };
            dist = chain.entry(prefix_key).or_default();
            dist.increment(next_tokens[*next_t as usize]);
        }
    }

    // Last window is also special
    if n_gram != 0 && window.len() > 0 {
        dist = chain.entry(bag.get_id(&window[1..]).unwrap()).or_default();
        dist.increment(bag.get_id(tokens::SpecialToken::End).unwrap());
    }
}

fn train_chains_from_files(bag: &wordbag::WordBag, chains: &mut tokens::TokenChains, files: &Vec<std::path::PathBuf>) {
    let next_tokens: [tokens::TokenId; u8::MAX as usize + 1] = std::array::from_fn(|i| bag.get_id(tokens::TokenValue::Next(i as u8)).unwrap());

    println!("train_chains_from_files:");
    for file_reader in io::get_file_readers(files).progress() {
        match file_reader {
            Ok(file_reader) => {
                let chains_iter = chains.par_iter_mut().filter(|(chain_id, _)| {
                    let n_gram = chain_id.0;
                    (CHAINS_START..=CTX_LEN).contains(&n_gram) && file_reader.len() >= n_gram + 1
                });
                #[cfg(feature = "experiment-n_gram-skips")]
                let chains_iter = chains_iter.filter(|(chain_id, _)| {
                    let n_gram = chain_id.0;
                    n_gram.is_power_of_two() || n_gram <= 1 || n_gram >= CTX_LEN
                });

                chains_iter.for_each(|(chain_id, chain)| {
                    train_chain_ngram(chain, bag, chain_id.0, next_tokens, &file_reader);
                });
            }
            Err(_) => unreachable!(),
        }
    }
}

fn sample_chains_backoff(bag: &wordbag::WordBag, chains: &tokens::TokenChains) {
    let mut rng: rand::rngs::SmallRng = rand::make_rng();
    let mut prefix_t: [u8; CTX_LEN] = [PREFILL_CT; CTX_LEN];
    let mut output: Vec<u8> = Vec::with_capacity(SAMPLE_SEQ);

    let token_zero_gram = bag.get_id(tokens::SpecialToken::ZeroGram).unwrap();
    let token_start = bag.get_id(tokens::SpecialToken::Start).unwrap();

    println!("sample_chains:");
    'sample: for _ in (0..SAMPLE_SEQ).progress() {
        'n_gram_iter: for n_gram in (CHAINS_START..=CTX_LEN).rev() {
            #[cfg(feature = "experiment-n_gram-skips")]
            if !n_gram.is_power_of_two() && 1 < n_gram && n_gram < CTX_LEN {
                continue 'n_gram_iter;
            };

            let this_prefix_t: tokens::TokenId;
            if n_gram == 0 {
                this_prefix_t = token_zero_gram;
            } else if prefix_t.iter().all(|&x| x == PREFILL_CT) {
                this_prefix_t = token_start;
            } else {
                match bag.get_id(prefix_t.get((CTX_LEN - n_gram)..).unwrap()) {
                    Some(id) => this_prefix_t = id,
                    None => continue 'n_gram_iter,
                }
            };
            let chain = chains.get(&tokens::ChainId(n_gram)).unwrap();
            if chain.is_empty() {
                continue 'n_gram_iter;
            };
            match chain.get(&this_prefix_t) {
                Some(dist) => {
                    match tests::sample(dist, &mut rng) {
                        Some(sample_t) => {
                            match bag.i2s(sample_t) {
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
                                None => todo!(),
                            };
                            break;
                        }
                        None => unreachable!(),
                    }
                }
                None => {
                    println!("Fuck.");

                    dbg!(n_gram);
                    dbg!(this_prefix_t);
                    dbg!(bag.i2s(this_prefix_t).unwrap());
                    dbg!(io::decode_chopped_utf8(&bag.i2s(this_prefix_t).unwrap().as_bytes()));

                    panic!("WOOPS: We shouldn't have got here!!! #1")
                }
            }
        }
    }

    println!();
    println!("SAMPLED:\n---\n\n{:}\n\n----\n", io::decode_chopped_utf8(&output));
    // println!("SAMPLED RAW:\t\t{:?}", &output);
    println!("PREFIX BUFFER STATE: {:?}", &prefix_t);
    println!("PREFIX BUFFER STATE (READABLE): {:?}", io::decode_chopped_utf8(&prefix_t));
    println!();
}

fn helper_add_to_vec(globs_s: &[&'static str]) -> Vec<std::path::PathBuf> {
    let mut f_list: Vec<std::path::PathBuf> = Vec::new();
    for &glob_s in globs_s {
        f_list.append(&mut glob::glob(glob_s).ok().into_iter().flatten().filter_map(Result::ok).collect());
    }
    f_list
}

fn main() {
    tests::run();

    let test_files = helper_add_to_vec(&[r"**/*.rs"]); // this wont find anything!

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
    bag.train_from_files(&test_files, CTX_LEN, PREFILL_CT);

    train_chains_from_files(&bag, &mut chains, &test_files);

    for _ in 0..NUM_SAMPLES {
        sample_chains_backoff(&bag, &chains);
    }
}
