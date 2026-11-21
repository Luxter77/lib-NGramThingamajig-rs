mod io;
mod tests;
mod tokens;
mod wordbag;

const CTX_LEN: usize = 5;
const TEST_INPUT_FILE: &'static str = r"src/main.rs";
const SAMPLE_SEQ: usize = 1000;

fn main() {
    tests::run();

    // init
    let mut chains = tokens::TokenChains::new();

    chains.insert(
        tokens::ChainId(0),
        std::collections::HashMap::with_capacity(1),
    );
    chains.insert(
        tokens::ChainId(1),
        std::collections::HashMap::with_capacity(u8::MAX as usize),
    );

    for n_gram in 2..=CTX_LEN {
        chains.insert(tokens::ChainId(n_gram), std::collections::HashMap::new());
    }

    let mut bag = wordbag::WordBag::with_capacity((u8::MAX as usize) * 5);
    bag.setup_special_tokens();
    let zero_gram = bag.s2i(tokens::SpecialToken::ZeroGram);

    for n_gram in 1..=CTX_LEN {
        for file_reader in io::get_file_readers(&[std::path::Path::new(TEST_INPUT_FILE)]) {
            match file_reader {
                Ok(file_reader) => {
                    for window in io::sliding_window_reader(&file_reader, n_gram) {
                        bag.s2i(window);
                    }
                }
                Err(_) => unreachable!(),
            }
        }
    }

    for file_reader in io::get_file_readers(&[std::path::Path::new(TEST_INPUT_FILE)]) {
        match file_reader {
            Ok(file_reader) => {
                for n_gram in 0..=CTX_LEN {
                    for window in io::sliding_window_reader(&file_reader, n_gram + 1) {
                        if let Some((next_t, prefix_t)) = window.split_last() {
                            let prefix_key = if n_gram == 0 {
                                zero_gram
                            } else {
                                bag.s2i(prefix_t)
                            };
                            chains
                                .get_mut(&tokens::ChainId(n_gram))
                                .unwrap()
                                .entry(prefix_key)
                                .or_default()
                                .increment(bag.s2i(std::slice::from_ref(next_t)));
                        }
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    // Sampling
    let mut rng = <rand::rngs::SmallRng as rand::SeedableRng>::seed_from_u64(42);
    let mut prefix_t: [u8; CTX_LEN] = [0u8; CTX_LEN];
    let mut output: Vec<u8> = Vec::with_capacity(SAMPLE_SEQ);

    for _ in 0..SAMPLE_SEQ {
        for n_gram in (0..=CTX_LEN).rev() {
            let this_prefix_t: tokens::TokenId;
            if n_gram == 0 {
                this_prefix_t = zero_gram;
            } else {
                let start = CTX_LEN - n_gram;
                let slice = prefix_t.get(start..).unwrap();
                if slice.iter().all(|&x| x == 0) {
                    this_prefix_t = zero_gram;
                } else {
                    match bag.get_id(slice) {
                        Some(id) => this_prefix_t = id,
                        None => continue,
                    }
                }
            };
            match chains
                .get(&tokens::ChainId(n_gram))
                .unwrap()
                .get(&this_prefix_t)
            {
                Some(dist) => {
                    match tests::sample(dist, &mut rng) {
                        Some(sample_t) => {
                            match bag.i2s(sample_t) {
                                Some(next_s) => {
                                    match next_s {
                                        tokens::TokenValue::Normal(items) => {
                                            match items.last() {
                                                Some(&new) => {
                                                    // this is correct don't fucking touch it
                                                    prefix_t[0] = new;
                                                    prefix_t.rotate_left(1);
                                                    output.push(new);
                                                }
                                                None => todo!(),
                                            }
                                        }
                                        tokens::TokenValue::Special(_) => {
                                            todo!()
                                        }
                                    }
                                }
                                None => todo!(),
                            };
                            break;
                        }
                        None => unreachable!(),
                    }
                }
                None => continue,
            }
        }
    }

    println!("SAMPLED: \n\n{}\n\n", decode_chopped_utf8(&output));
    println!("SAMPLED RAW: \n\n{:?}\n\n", &output);
    println!("PREFIX BUFFER STATE: \n\n{:?}\n\n", &prefix_t);
}

fn decode_chopped_utf8(bytes: &[u8]) -> String {
    let mut decoder = encoding_rs::UTF_8.new_decoder();

    let mut output = String::with_capacity(bytes.len());

    let (_result, _bytes_read, _had_errors) = decoder.decode_to_string(bytes, &mut output, false);

    output
}
