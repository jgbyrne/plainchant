use crate::site::Feather;
use crate::util;
use chrono::{TimeZone, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;

pub fn utc_timestamp(ts: u64) -> String {
    Utc.timestamp(ts.try_into().unwrap_or(0), 0).to_string()
}

const SECS_IN_YEAR: u64 = 365 * 24 * 60 * 60;
const SECS_IN_DAY: u64 = 24 * 60 * 60;
const SECS_IN_HOUR: u64 = 60 * 60;
const SECS_IN_MIN: u64 = 60;

pub fn humanise_time(ts: u64) -> String {
    let cur_ts = util::timestamp();
    let delta = cur_ts - ts;

    let years = delta / SECS_IN_YEAR;
    if years == 1 {
        return String::from("1 year ago");
    } else if years > 1 {
        return format!("{} years ago", years);
    }

    let days = delta / SECS_IN_DAY;
    if days == 1 {
        return String::from("1 day ago");
    } else if days > 1 {
        return format!("{} days ago", days);
    }

    let hours = delta / SECS_IN_HOUR;
    if hours == 1 {
        return String::from("1 hour ago");
    } else if hours > 1 {
        return format!("{} hours ago", hours);
    }

    let mins = delta / SECS_IN_MIN;
    if mins == 1 {
        return String::from("1 minute ago");
    } else if mins > 1 {
        return format!("{} minutes ago", mins);
    }

    String::from("less than a minute ago")
}

pub fn annotate_post(body: &str, posts: &HashSet<u64>) -> String {
    lazy_static! {
        // Match quoted (greentext) lines
        static ref QUOTED: Regex = Regex::new(r"^\s*>(?:$|[^>])").unwrap();
        // Capture replies of form >>390290 and >>>/blah/2939404
        static ref REPLY: Regex = Regex::new(r"(>>)(?:([\d]+)|>/(\w+)/([\d]+))(?:$|\W)").unwrap();
    }

    let mut out = String::new();
    for line in body.lines() {
        let quoted = QUOTED.is_match(line);

        if quoted {
            out.push_str("<span class='quote'>");
        }
        let mut left = 0;

        for reply in REPLY.captures_iter(line) {
            let start = reply.get(1).unwrap().start();

            out.push_str(&line[left..start]);

            let local = match reply.get(3) {
                Some(_) => false,
                None => true,
            };
            if local {
                let post_num = reply.get(2).unwrap();
                let right = post_num.end();

                if let Ok(num) = &post_num.as_str().parse::<u64>() {
                    // If the post is in the same thread, we link to the
                    // anchor so the browser need not make a request.
                    // Otherwise we link directly to the post.
                    let link = match posts.contains(&num) {
                        true => format!("#{}", num),
                        false => format!("./{}", num),
                    };
                    out.push_str(&format!("<a href='{}'>&gt;&gt;{}</a>", &link, num));
                } else {
                    out.push_str(&line[start..right]);
                }

                left = right;
            } else {
                let board_id = reply.get(3).unwrap();
                let post_num = reply.get(4).unwrap();
                let right = post_num.end();

                let url = board_id.as_str();
                let parsed_post_num = post_num.as_str().parse::<u64>();
                let board_plausible = url.len() < 7 && url.chars().all(|c| c.is_alphanumeric());

                if let (Ok(num), true) = (&parsed_post_num, board_plausible) {
                    // TODO: This link might be flaky (should it really be absolute path?)
                    out.push_str(&format!(
                        "<a href='/{0}/thread/{1}'>&gt;&gt;&gt;/{0}/{1}</a>",
                        &url, &num
                    ));
                } else {
                    out.push_str(&line[start..right]);
                }

                left = right;
            }
        }

        out.push_str(&line[left..]);

        if quoted {
            out.push_str("</span>");
        }

        out.push_str("\n");
    }
    out
}

pub fn display_feather(feather: &Feather) -> String {
    match feather {
        Feather::None => String::from(""),
        Feather::Trip(s) => format!("# {}", s),
        Feather::Moderator => String::from("(Moderator)"),
        Feather::Admin => String::from("(Admin)"),
    }
}

pub fn html_escape(text: &str) -> String {
    let mut buf = String::new();
    for c in text.chars() {
        match c {
            '<' => buf.push_str("&lt;"),
            '>' => buf.push_str("&gt;"),
            '&' => buf.push_str("&amp;"),
            c @ _ => buf.push(c),
        }
    }
    buf
}
