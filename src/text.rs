use std::ops::RangeInclusive;

pub fn decode_chopped_utf8(bytes: &[u8]) -> String {
    let mut decoder = encoding_rs::UTF_8.new_decoder();

    let mut output = String::with_capacity(bytes.len());

    let (_result, _bytes_read, _had_errors) = decoder.decode_to_string(bytes, &mut output, false);

    output
}

/// Returns the valid next-byte ranges that keep `bytes` a valid UTF-8 prefix.
///
/// Assumes `bytes` is already a valid UTF-8 prefix (complete or mid-character).
/// Inspects at most the last 4 bytes in O(1) time without allocating.
/// Returns an empty slice if the trailing sequence is malformed.
pub fn valid_next_utf8_bytes(bytes: &[u8]) -> &'static [RangeInclusive<u8>] {
    const BOUNDARY: &[RangeInclusive<u8>] = &[0x00..=0x7F, 0xC2..=0xF4];
    const CONT_ANY: &[RangeInclusive<u8>] = &[0x80..=0xBF];
    const CONT_E0: &[RangeInclusive<u8>] = &[0xA0..=0xBF];
    const CONT_ED: &[RangeInclusive<u8>] = &[0x80..=0x9F];
    const CONT_F0: &[RangeInclusive<u8>] = &[0x90..=0xBF];
    const CONT_F4: &[RangeInclusive<u8>] = &[0x80..=0x8F];

    if bytes.is_empty() {
        return BOUNDARY;
    }

    let cont_count = bytes.iter().rev().take(3).take_while(|&&b| (b & 0xC0) == 0x80).count();

    let Some(lead_idx) = bytes.len().checked_sub(cont_count + 1) else {
        return &[];
    };

    match (cont_count, bytes[lead_idx]) {
        // Complete 1-, 2-, 3-, or 4-byte character: expect a new leading byte
        (0, 0x00..=0x7F) | (1, 0xC2..=0xDF) | (2, 0xE0..=0xEF) | (3, 0xF0..=0xF4) => BOUNDARY,

        // Waiting for byte 2 of a multi-byte character
        (0, 0xC2..=0xDF | 0xE1..=0xEC | 0xEE..=0xEF | 0xF1..=0xF3) => CONT_ANY,
        (0, 0xE0) => CONT_E0,
        (0, 0xED) => CONT_ED,
        (0, 0xF0) => CONT_F0,
        (0, 0xF4) => CONT_F4,

        // Waiting for byte 3 or byte 4 of a multi-byte character
        (1, 0xE0..=0xEF) | (1 | 2, 0xF0..=0xF4) => CONT_ANY,

        // Malformed trailing sequence
        _ => &[],
    }
}

/// Returns true if appending `next_t` to `prefix_t` produces a valid UTF-8 prefix.
///
/// Assumes `prefix_t` is already a valid UTF-8 prefix (complete or mid-character).
/// Inspects at most the last 4 bytes in O(1) time without allocating.
pub fn is_valid_next_utf8_byte(next_t: u8, prefix_t: &[u8]) -> bool {
    if prefix_t.is_empty() {
        return matches!(next_t, 0x00..=0x7F | 0xC2..=0xF4);
    }

    let cont_count = prefix_t.iter().rev().take(3).take_while(|&&b| (b & 0xC0) == 0x80).count();

    let Some(lead_idx) = prefix_t.len().checked_sub(cont_count + 1) else {
        return false;
    };

    matches!(
        (cont_count, prefix_t[lead_idx], next_t),
        // Complete character: next byte must start a new character
        (0, 0x00..=0x7F, 0x00..=0x7F | 0xC2..=0xF4)
            | (1, 0xC2..=0xDF, 0x00..=0x7F | 0xC2..=0xF4)
            | (2, 0xE0..=0xEF, 0x00..=0x7F | 0xC2..=0xF4)
            | (3, 0xF0..=0xF4, 0x00..=0x7F | 0xC2..=0xF4)
            // Byte 2 of a multi-byte character
            | (0, 0xC2..=0xDF | 0xE1..=0xEC | 0xEE..=0xEF | 0xF1..=0xF3, 0x80..=0xBF)
            | (0, 0xE0, 0xA0..=0xBF)
            | (0, 0xED, 0x80..=0x9F)
            | (0, 0xF0, 0x90..=0xBF)
            | (0, 0xF4, 0x80..=0x8F)
            // Byte 3 or byte 4 of a multi-byte character
            | (1, 0xE0..=0xEF, 0x80..=0xBF)
            | (1 | 2, 0xF0..=0xF4, 0x80..=0xBF)
    )
}
