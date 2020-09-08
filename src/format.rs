use std::collections::HashSet;

pub fn annotate_post(body: &str, posts: &HashSet<u64>) -> String {
    let mut out = String::new();
    for line in body.lines() {
        let mut c_iter = line.chars();
        match c_iter.next() {
            Some('>') => {
                let is_link = match c_iter.next() {
                    Some('>') => {
                        match c_iter.next() {
                            Some('>') => {
                                // TODO
                                out.push_str(line);
                                true
                            },
                            Some(n) => {
                                if !n.is_ascii_digit() {
                                    false
                                } else {
                                    let mut num = n.to_string();
                                    let mut post = String::new();

                                    let mut is_link = true;
                                    while let Some(c) = c_iter.next() {
                                        if c.is_ascii_digit() {
                                            num.push(c);
                                        } else if !c.is_whitespace() {
                                            is_link = false;
                                            break;
                                        } else {
                                            post.push(c);
                                            break;
                                        }
                                    }

                                    if is_link {
                                        if let Ok(num) = num.parse::<u64>() {
                                            post.push_str(&c_iter.collect::<String>());

                                            // If the post is in the same thread, we link to the
                                            // anchor so the browser need not make a request.
                                            // Otherwise we link directly to the post.
                                            let link = match posts.contains(&num) {
                                                true => format!("#{}", num),
                                                false => format!("./{}", num),
                                            };
                                            out.push_str(&format!("<a href='{}'>>>{}</a>", &link, num));
                                            out.push_str(&post);
                                            true
                                        }
                                        else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                }
                            },
                            None => false,
                        }
                    },
                    _ => false,
                };

                if !is_link {
                    out.push_str("<span class='quote'>");
                    out.push_str(line);
                    out.push_str("</span>");
                }
            },
            _ => {
                out.push_str(line);
            },
        }
        out.push('\n');
    }
    out
}
