pub(crate) fn compute_name_browse_sort_key(name: &str) -> String {
    const VERSION_BYTE: u8 = 0x01;
    const TEXT_SEGMENT: u8 = 0x02;
    const DIGIT_SEGMENT: u8 = 0x03;
    const ORIGINAL_LENGTH: u8 = 0x05;
    const END_MARKER: u8 = 0x04;

    let mut key = Vec::with_capacity(name.len() * 3);
    key.push(VERSION_BYTE);

    let mut chars = name.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() {
            let mut digit_str = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    digit_str.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            let numeric_value: i64 = digit_str.parse().unwrap_or(0);
            key.push(DIGIT_SEGMENT);
            write_zero_padded_i64(&mut key, numeric_value);
            key.push(ORIGINAL_LENGTH);
            key.push(digit_str.len() as u8);
            key.extend_from_slice(digit_str.as_bytes());
        } else {
            key.push(TEXT_SEGMENT);
            for lower_ch in ch.to_lowercase() {
                key.extend_from_slice(lower_ch.to_string().as_bytes());
            }
            chars.next();
        }
    }

    key.push(END_MARKER);
    String::from_utf8(key).expect("browse sort key must be valid UTF-8")
}

fn write_zero_padded_i64(buf: &mut Vec<u8>, value: i64) {
    const WIDTH: usize = 19;
    let unsigned = if value < 0 {
        buf.push(b'-');
        (value as u128).wrapping_neg() as u64
    } else {
        value as u64
    };
    let divisor = 1_000_000_000_000_000_000u64;
    let mut remaining = unsigned;
    for _ in 0..WIDTH {
        let digit = (remaining / divisor) as u8;
        buf.push(b'0' + digit);
        remaining = (remaining % divisor) * 10;
    }
}

#[cfg(test)]
mod tests {
    use super::compute_name_browse_sort_key;

    fn assert_order(expected: &[&str]) {
        let keys: Vec<(String, &str)> = expected
            .iter()
            .map(|name| (compute_name_browse_sort_key(name), *name))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        let result: Vec<&&str> = sorted.iter().map(|(_, name)| name).collect();
        let expected_refs: Vec<&&str> = expected.iter().collect();
        assert_eq!(result, expected_refs, "natural sort order mismatch");
    }

    #[test]
    fn natural_sort_orders_digits_numerically() {
        assert_order(&["[1]", "[2]", "[10]"]);
    }

    #[test]
    fn natural_sort_orders_track_numbers() {
        assert_order(&["Track 1.wav", "Track 2.wav", "Track 10.wav"]);
    }

    #[test]
    fn natural_sort_case_insensitive_primary() {
        let key_lower = compute_name_browse_sort_key("track 1.wav");
        let key_upper = compute_name_browse_sort_key("Track 1.wav");
        assert_eq!(
            key_lower, key_upper,
            "case-folded keys must be identical for deterministic tie-breaking by id"
        );
    }

    #[test]
    fn natural_sort_leading_zeros_deterministic() {
        let key_01 = compute_name_browse_sort_key("Track 01.wav");
        let key_1 = compute_name_browse_sort_key("Track 1.wav");
        let key_001 = compute_name_browse_sort_key("Track 001.wav");
        assert!(
            key_1 < key_01,
            "1 should sort before 01 (shorter digit string first)"
        );
        assert!(
            key_01 < key_001,
            "01 should sort before 001 (shorter digit string first)"
        );
    }

    #[test]
    fn natural_sort_mixed_text_and_digits() {
        assert_order(&[
            "1 Introduction",
            "2 The Beginning",
            "10 The Climax",
            "11 The Resolution",
        ]);
    }

    #[test]
    fn natural_sort_pure_text() {
        assert_order(&["Aardvark", "Apple", "Banana", "zebra"]);
    }

    #[test]
    fn natural_sort_empty_string() {
        let key = compute_name_browse_sort_key("");
        assert!(!key.is_empty(), "empty name must produce a valid key");
    }

    #[test]
    fn natural_sort_multiple_digit_runs() {
        assert_order(&[
            "Disc 1 - Track 2",
            "Disc 1 - Track 10",
            "Disc 2 - Track 1",
            "Disc 10 - Track 1",
        ]);
    }

    #[test]
    fn natural_sort_no_digits() {
        let key_a = compute_name_browse_sort_key("aaa");
        let key_b = compute_name_browse_sort_key("bbb");
        assert!(key_a < key_b, "pure text keys must sort alphabetically");
    }

    #[test]
    fn natural_sort_zero_value() {
        assert_order(&["Track 0.wav", "Track 1.wav", "Track 10.wav"]);
    }

    #[test]
    fn natural_sort_negative_stability() {
        let names = ["[10]", "[1]", "[2]", "[20]", "[3]", "[11]"];
        let mut keyed: Vec<_> = names
            .iter()
            .map(|n| (compute_name_browse_sort_key(n), *n))
            .collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0));
        let sorted: Vec<&&str> = keyed.iter().map(|(_, n)| n).collect();
        assert_eq!(sorted, vec![&"[1]", &"[2]", &"[3]", &"[10]", &"[11]", &"[20]"]);
    }
}