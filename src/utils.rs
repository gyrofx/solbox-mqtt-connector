pub fn shorten_string(string: &str, max_length: usize) -> String {
    if string.len() <= max_length {
        return string.to_string();
    }

    let mut shortened = String::from(string);
    shortened.truncate(max_length);
    shortened.push_str("...");
    shortened
}
