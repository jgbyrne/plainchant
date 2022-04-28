use std::collections::HashSet;
use crate::util;
use chrono::{Utc, TimeZone};

pub fn utc_timestamp(ts: u64) -> String {
    Utc.timestamp(ts.try_into().unwrap_or(0), 0).to_string()
}

const SECS_IN_YEAR : u64 = 365 * 24 * 60 * 60;
const SECS_IN_DAY  : u64 = 24 * 60 * 60;
const SECS_IN_HOUR : u64 = 60 * 60;
const SECS_IN_MIN  : u64 = 60;

pub fn humanise_time(ts: u64) -> String {
    let cur_ts = util::timestamp();
    let delta = cur_ts - ts;

    let years = delta / SECS_IN_YEAR;
    if years == 1 {
        return String::from("1 year ago");
    }
    else if years > 1 {
        return format!("{} years ago", years);
    }

    let days  = delta / SECS_IN_DAY;
    if days == 1 {
        return String::from("1 day ago");
    }
    else if days > 1 {
        return format!("{} days ago", days);
    }

    let hours = delta / SECS_IN_HOUR;
    if hours == 1 {
        return String::from("1 hour ago");
    }
    else if hours > 1 {
        return format!("{} hours ago", hours);
    }

    let mins = delta / SECS_IN_MIN;
    if mins == 1 {
        return String::from("1 minute ago");
    }
    else if mins > 1 {
        return format!("{} minutes ago", mins);
    }
    
    String::from("less than a minute ago")
}


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
