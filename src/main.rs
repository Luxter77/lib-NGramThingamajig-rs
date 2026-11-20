use std::path::Path;

mod wordbag;
mod tokens;
mod tests;
mod io;

const CTX_LEN: usize = 5;
const TEST_INPUT_FILE: &'static str = r"src/main.rs";

fn main() {
    tests::run();

    // init
    let mut chains = tokens::TokenChains::new();

    chains.insert(tokens::ChainId(0), std::collections::HashMap::with_capacity(1));
    chains.insert(tokens::ChainId(1), std::collections::HashMap::with_capacity(u8::MAX as usize));

    for n_gram in 2..=CTX_LEN {
        chains.insert(tokens::ChainId(n_gram), std::collections::HashMap::new());
    }

    let mut bag = wordbag::WordBag::with_capacity((u8::MAX as usize) * 5);
    bag.setup_special_tokens();
    let zero_gram = bag.s2i(tokens::SpecialToken::ZeroGram);

    for n_gram in 1..=CTX_LEN {
        for file_reader in io::get_file_readers(&[Path::new(TEST_INPUT_FILE)]) {
            match file_reader {
                Ok(file_reader) => {
                    for window in io::sliding_window_reader(&file_reader, n_gram) {
                        bag.s2i(window);
                    }
                },
                Err(_) => unreachable!()
            }
        }
    }

    for file_reader in io::get_file_readers(&[Path::new(TEST_INPUT_FILE)]) {
        match file_reader {
            Ok(file_reader) => {
                for n_gram in 1..=CTX_LEN {
                    for window in io::sliding_window_reader(&file_reader, n_gram + 1) {
                        if let Some((next_t , prefix_t)) = window.split_last() {
                            chains
                                .get_mut(&tokens::ChainId(n_gram))
                                .unwrap()
                                .entry(match n_gram {
                                    0 => zero_gram,
                                    _ => bag.s2i(prefix_t)
                                }
                                )
                                .or_default()
                                .increment(bag.s2i(std::slice::from_ref(next_t)));
                        }
                    }
                }
            },
            Err(_) => unreachable!()
        }
    }

}
