pub fn annotate_post(body: &str) -> String {
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
                                }
                                else {
                                    let mut num = n.to_string();
                                    let mut post = String::new();
                                    
                                    let mut is_not_link = false;
                                    while let Some(c) = c_iter.next() {
                                        if c.is_ascii_digit() {
                                            num.push(c);
                                        }
                                        else if !c.is_whitespace() {
                                            is_not_link = true;
                                            break;
                                        }
                                        else {
                                           post.push(c);
                                           break;
                                        }
                                    }

                                    if is_not_link {
                                        false
                                    }
                                    else {
                                        post.push_str(&c_iter.collect::<String>());

                                        out.push_str(&format!("<a href='#{}'>", num));
                                        out.push_str(">>");
                                        out.push_str(&num);
                                        out.push_str("</a>");
                                        out.push_str(&post);
                                        true
                                    }
                                }
                            },
                            None => false,
                        }
                    }
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
