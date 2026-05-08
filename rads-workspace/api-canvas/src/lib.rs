#[cfg(test)]
mod tests {
    use super::reverse_string;

    #[test]
    fn reverses_ascii_string() {
        assert_eq!(reverse_string("abc"), "cba");
    }

    #[test]
    fn reverses_unicode_string() {
        assert_eq!(reverse_string("héllo"), "olléh");
    }
}

pub fn reverse_string(input: &str) -> String {
    input.chars().rev().collect()
}
