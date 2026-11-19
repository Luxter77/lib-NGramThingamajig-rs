mod tokens;

fn main() {
    println!("Size of Message: {} bytes", std::mem::size_of::<tokens::TokenValue>());
    // You can also check it at compile time:
    const SIZE: usize = std::mem::size_of::<tokens::TokenValue>();
    assert_eq!(SIZE, 24); // Rust uses niche optimization on String's pointer!
}
