pub fn count_to_10() -> Vec<u8> {
    (1..=10).collect()
}

pub fn reverse_string(input: &str) -> String {
    input.chars().rev().collect()
}

pub fn coordinates_2d(x: i32, y: i32) -> (i32, i32) {
    (x, y)
}
