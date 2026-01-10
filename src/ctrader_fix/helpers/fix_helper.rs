pub fn parse_fix_field(field: &str) -> Option<(u32, &str)> {
    // split_once splits the string at the first occurrence of '='
    // and_then will run the closure if the || equals to Some(T), if it is None, then skip the closure and return none
    field.split_once('=').and_then(|(tag_str, value)| {
        tag_str.parse::<u32>().ok().map(|tag| (tag, value))
    })
}
