pub fn annotate_post(body: &str) -> String {
    let mut out = String::new();
    for line in body.lines() {
        match line.chars().next() {
            Some('>') => {
                out.push_str("<span class='quote'>");
                out.push_str(line);
                out.push_str("</span>");
            },
            _ => {
                out.push_str(line);
            },
        }
        out.push('\n');
    }
    out
}
