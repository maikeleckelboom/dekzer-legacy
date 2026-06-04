pub(crate) fn compute_name_browse_sort_key(name: &str) -> String {
    let mut key = String::with_capacity(name.len() * 4 + 8);
    key.push_str("v1|");

    let mut chars = name.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() {
            let mut digit_chars = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    digit_chars.push(d);
                    chars.next();
                } else {
                    break;
                }
            }

            let significant = strip_leading_zeros(&digit_chars);
            write_digit_run(&mut key, &significant, &digit_chars);
        } else {
            let mut text_chars = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() {
                    break;
                }
                for lower_ch in c.to_lowercase() {
                    text_chars.push(lower_ch);
                }
                chars.next();
            }
            write_text_chars(&mut key, &text_chars);
        }
    }

    key
}

fn strip_leading_zeros(s: &str) -> String {
    let trimmed = s.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

fn write_text_chars(key: &mut String, text: &str) {
    for ch in text.chars() {
        key.push('t');
        key.push(ch);
    }
}

fn write_digit_run(key: &mut String, significant: &str, original: &str) {
    key.push('d');
    key.push_str(&format!("{:05}", significant.len()));
    key.push(':');
    key.push_str(significant);
    key.push(':');
    key.push_str(&format!("{:05}", original.len()));
    key.push(':');
    key.push_str(original);
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
    fn natural_sort_orders_leading_zeros_deterministic() {
        assert_order(&["[1]", "[01]", "[001]", "[2]", "[10]"]);
    }

    #[test]
    fn natural_sort_orders_track_numbers() {
        assert_order(&["Track 1.wav", "Track 2.wav", "Track 10.wav"]);
    }

    #[test]
    fn natural_sort_orders_track_numbers_with_leading_zeros() {
        assert_order(&[
            "Track 1.wav",
            "Track 01.wav",
            "Track 001.wav",
            "Track 2.wav",
            "Track 10.wav",
        ]);
    }

    #[test]
    fn natural_sort_leading_zeros_deterministic_individual() {
        let key_01 = compute_name_browse_sort_key("Track 01.wav");
        let key_1 = compute_name_browse_sort_key("Track 1.wav");
        let key_001 = compute_name_browse_sort_key("Track 001.wav");
        assert!(
            key_1 < key_01,
            "1 should sort before 01 (shorter original digit string first)"
        );
        assert!(
            key_01 < key_001,
            "01 should sort before 001 (shorter original digit string first)"
        );
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

    #[test]
    fn large_number_does_not_overflow_and_sorts_after_smaller() {
        let key_9 = compute_name_browse_sort_key("Track 9.wav");
        let key_10 = compute_name_browse_sort_key("Track 10.wav");
        let key_huge = compute_name_browse_sort_key(
            "Track 999999999999999999999999999999.wav",
        );
        assert!(
            key_9 < key_10,
            "Track 9 must sort before Track 10"
        );
        assert!(
            key_10 < key_huge,
            "Track 10 must sort before huge-number Track"
        );
        assert!(
            key_9 < key_huge,
            "Track 9 must sort before huge-number Track"
        );
    }

    #[test]
    fn large_number_with_many_leading_zeros_sorts_by_numeric_value_9() {
        let key_9 = compute_name_browse_sort_key("Track 9.wav");
        let key_padded = compute_name_browse_sort_key(
            "Track 000000000000000000000000000009.wav",
        );
        let key_10 = compute_name_browse_sort_key("Track 10.wav");

        assert!(
            key_9 < key_padded,
            "Track 9 must sort before padded-zero variant (shorter original length wins)"
        );
        assert!(
            key_padded < key_10,
            "padded-zero variant (value 9) must sort before Track 10"
        );
    }

    #[test]
    fn all_zero_digit_run_collapses_to_single_zero() {
        let key_000 = compute_name_browse_sort_key("Track 000.wav");
        let key_0 = compute_name_browse_sort_key("Track 0.wav");
        assert!(
            key_0 < key_000,
            "Track 0 must sort before Track 000 (shorter original length)"
        );
        assert_order(&["Track 0.wav", "Track 000.wav", "Track 1.wav"]);
    }

    #[test]
    fn key_is_printable_ascii() {
        let key = compute_name_browse_sort_key("Track 10.wav");
        assert!(key.chars().all(|c| c.is_ascii_graphic() || c == ' ' || c == ':'));
        assert!(key.starts_with("v1|"));
    }

    #[test]
    fn sort_key_never_empty_for_any_valid_input() {
        let names = [
            "",
            "a",
            "1",
            "[1]",
            "Track 1.wav",
            "a very long file name with spaces and 123 numbers.mp3",
            "0",
            "000",
        ];
        for name in &names {
            let key = compute_name_browse_sort_key(name);
            assert!(
                !key.is_empty(),
                "sort key must not be empty for name: {name:?}"
            );
            assert!(
                key.starts_with("v1|"),
                "sort key must have v1| prefix for name: {name:?}"
            );
        }
    }
}
