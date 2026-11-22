mod io;
mod tests;
mod tokens;
mod wordbag;

use zzz::ProgressBarIterExt as _;
use glob::glob;

const CTX_LEN:    usize = 7;
const SAMPLE_SEQ: usize = 10000;

fn train_wordbag_from_files(bag: &mut wordbag::WordBag, files: &Vec<std::path::PathBuf>) {
    println!("train_wordbag_from_files:");
    for n_gram in (1..=CTX_LEN).progress() {
        for file_reader in io::get_file_readers(files).progress() {
            match file_reader {
                Ok(file_reader) => {
                    for window in io::sliding_window_reader(&file_reader, n_gram) {
                        bag.s2i(tokens::TokenValue::Normal(window.into()));
                    }
                }
                Err(_) => unreachable!(),
            }
        }
    }
}

fn train_chains_from_files(bag: &mut wordbag::WordBag, chains: &mut tokens::TokenChains, files: &Vec<std::path::PathBuf>) {
    let token_zero_gram = bag.s2i(tokens::SpecialToken::ZeroGram);
    let token_start = bag.s2i(tokens::SpecialToken::Start);

    println!("train_chains_from_files:");
    for file_reader in io::get_file_readers(files).progress() {
        match file_reader {
            Ok(file_reader) => {
                for n_gram in 0..=CTX_LEN {
                    let mut window_iterator = io::sliding_window_reader(&file_reader, n_gram + 1);
                    let mut window: &[u8];

                    let chain: &mut tokens::TokenChain = chains.get_mut(&tokens::ChainId(n_gram)).unwrap();
                    let mut dist: &mut tokens::TokenDist = &mut tokens::TokenDist::Empty;

                    // First window is special
                    if n_gram != 0 {
                        if let Some(i_window) = window_iterator.next() {
                            window = i_window;
                            chain.entry(token_start).or_default().increment(bag.s2i(window.first().unwrap()));

                            // Normal window proc
                            if let Some((next_t, prefix_t)) = window.split_last() {
                                let prefix_key = bag.s2i(prefix_t);
                                dist = chain.entry(prefix_key).or_default();
                                dist.increment(bag.s2i(tokens::TokenValue::Next(*next_t)));
                            }
                        }
                    }

                    // Rest of the windows
                    for i_window in window_iterator {
                        window = i_window;
                        if let Some((next_t, prefix_t)) = window.split_last() {
                            let prefix_key = if n_gram == 0 { token_zero_gram } else { bag.s2i(prefix_t) };
                            dist = chain.entry(prefix_key).or_default();
                            dist.increment(bag.s2i(tokens::TokenValue::Next(*next_t)));
                        }
                    }

                    // Last window is also special
                    if n_gram != 0 {
                        dist.increment(bag.s2i(tokens::SpecialToken::End));
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }
}

fn sample_chains(bag: &mut wordbag::WordBag, chains: &mut tokens::TokenChains) {
    let mut rng: rand::rngs::SmallRng = rand::make_rng();
    let mut prefix_t: [u8; CTX_LEN] = [0; CTX_LEN];
    let mut output: Vec<u8> = Vec::with_capacity(SAMPLE_SEQ);

    let token_zero_gram = bag.s2i(tokens::SpecialToken::ZeroGram);
    let token_start = bag.s2i(tokens::SpecialToken::Start);

    println!("sample_chains:");
    'sample: for _ in (0..SAMPLE_SEQ).progress() {
        'n_gram_iter: for n_gram in (0..=CTX_LEN).rev() {
            let this_prefix_t: tokens::TokenId;
            if n_gram == 0 {
                this_prefix_t = token_zero_gram;
            } else if prefix_t.iter().all(|&x| x == 0) {
                this_prefix_t = token_start;
            } else {
                match bag.get_id(prefix_t.get((CTX_LEN - n_gram)..).unwrap()) {
                    Some(id) => this_prefix_t = id,
                    None => continue 'n_gram_iter,
                }
            };
            match chains.get(&tokens::ChainId(n_gram)).unwrap().get(&this_prefix_t) {
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
                None => unreachable!(),
            }
        }
    }

    println!();
    println!("SAMPLED:\t\t{:?}", io::decode_chopped_utf8(&output));
    println!("SAMPLED RAW:\t\t{:?}", &output);
    println!("PREFIX BUFFER STATE:\t{:?}", &prefix_t);
    println!();
}

fn main() {
    tests::run();

    let mut test_files: Vec<std::path::PathBuf> = Vec::new();
    test_files.append(&mut glob("**/*.rs").ok().into_iter().flatten().filter_map(Result::ok).collect());

    // init
    let mut chains: tokens::TokenChains = tokens::TokenChains::new();

    chains.insert(tokens::ChainId(0), std::collections::HashMap::with_capacity(1));
    chains.insert(tokens::ChainId(1), std::collections::HashMap::with_capacity(u8::MAX as usize));

    for n_gram in 2..=CTX_LEN {
        chains.insert(tokens::ChainId(n_gram), std::collections::HashMap::new());
    }

    let mut bag = wordbag::WordBag::with_capacity((u8::MAX as usize) * 5);

    bag.setup_special_tokens();

    train_wordbag_from_files(&mut bag, &test_files);
    train_chains_from_files(&mut bag, &mut chains, &test_files);

    sample_chains(&mut bag, &mut chains);
}
