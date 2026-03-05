use crate::actions;
use crate::db;
use crate::fr;
use crate::site;
use crate::site::Post;
use crate::util;
use std::sync::Arc;

pub fn execute<DB, FR>(
    actions: Arc<actions::Actions>,
    database: Arc<DB>,
    file_rack: Arc<FR>,
    command: &str,
) -> String
where
    DB: 'static + db::Database + Sync + Send,
    FR: 'static + fr::FileRack + Sync + Send,
{
    let parts = command.split(" ").collect::<Vec<&str>>();
    if parts.is_empty() {
        return String::from("");
    }

    match parts[0] {
        "boards" => {
            let boards = database.get_boards().unwrap_or_else(|_| vec![]);
            let mut outbuf = String::new();

            for board in boards {
                outbuf += &format!("{} - {} - {}\n", board.id, board.url, board.title);
            }

            outbuf
        },

        "post" => {
            if parts.len() < 4 {
                return String::from(
                    "post (show|rm|approve|mod|admin|nocap) <board_id> <post_num>\n",
                );
            }

            let board_id = match parts[2].parse::<u64>() {
                Ok(id) => id,
                Err(_) => {
                    return String::from("Board ID did not parse");
                },
            };

            let post_num = match parts[3].parse::<u64>() {
                Ok(id) => id,
                Err(_) => {
                    return String::from("Post num did not parse");
                },
            };

            match parts[1] {
                "show" => match database.get_post(board_id, post_num) {
                    Ok(post) => format!(
                        "Post {}/#{}\nIP: {} | Time: {}\n{}\n",
                        board_id,
                        post_num,
                        post.ip(),
                        post.time(),
                        &post.body().chars().take(256).collect::<String>()
                    ),
                    Err(err) => format!("Error: {:?}\n", err),
                },
                "rm" => {
                    match actions.delete_post(
                        database.as_ref(),
                        file_rack.as_ref(),
                        board_id,
                        post_num,
                    ) {
                        Ok(_) => String::from("Post deleted\n"),
                        Err(err) => format!("Error: {:?}\n", err),
                    }
                },
                "approve" => match database.get_post(board_id, post_num) {
                    Ok(mut post) => {
                        post.set_approval(site::Approval::Approved);
                        match database.update_post(post) {
                            Ok(_) => String::from("Approved Post\n"),
                            Err(err) => format!("Error: {:?}\n", err),
                        }
                    },
                    Err(err) => format!("Error: {:?}\n", err),
                },
                cmd @ ("mod" | "admin" | "nocap") => {
                    let feather = match cmd {
                        "mod" => site::Feather::Moderator,
                        "admin" => site::Feather::Admin,
                        "nocap" => site::Feather::None,
                        _ => unreachable!(),
                    };

                    match database.get_post(board_id, post_num) {
                        Ok(mut post) => {
                            post.set_feather(feather);

                            match database.update_post(post) {
                                Ok(_) => format!("Added {} feather to post\n", cmd),
                                Err(err) => format!("Error: {:?}\n", err),
                            }
                        },
                        Err(err) => format!("Error: {:?}\n", err),
                    }
                },
                _ => String::from("?\n"),
            }
        },

        "ban" => {
            if parts.len() < 3 {
                return String::from("ban show <ip>, ban add <ip>, ban rm <ip>\n");
            }
            let ip = parts[2].trim();
            match parts[1] {
                "show" => match actions.is_banned(ip, util::timestamp()) {
                    Ok(banned) => format!("IP {} ban status: {}\n", ip, banned),
                    Err(err) => format!("Error: {:?}\n", err),
                },
                "add" => match actions.ban_ip(database.as_ref(), ip, 300_000_000) {
                    Ok(_) => format!("Banned IP: {}\n", ip),
                    Err(err) => format!("Error: {:?}\n", err),
                },
                "rm" => match actions.unban_ip(database.as_ref(), ip) {
                    Ok(_) => format!("Un-banned IP: {}\n", ip),
                    Err(err) => format!("Error: {:?}\n", err),
                },
                _ => String::from("?\n"),
            }
        },

        "purge" => {
            if parts.len() < 3 {
                return String::from("purge (dry|exec) <board_id> <post_num>\n");
            }

            let dry_run = match parts[1] {
                "dry" => true,
                "exec" => false,
                _ => {
                    return String::from("?\n");
                },
            };

            let board_id = match parts[2].parse::<u64>() {
                Ok(id) => id,
                Err(_) => {
                    return String::from("Board ID did not parse");
                },
            };

            let post_num = match parts[3].parse::<u64>() {
                Ok(id) => id,
                Err(_) => {
                    return String::from("Post num did not parse");
                },
            };

            let mut str_out = String::new();

            let post_ip = match database.get_post(board_id, post_num) {
                Ok(post) => post.ip().to_owned(),
                Err(_) => {
                    str_out.push_str("Could not get post");
                    return str_out;
                },
            };

            if dry_run {
                str_out.push_str(&format!("Would Ban IP: {}\n", &post_ip));
            } else {
                match actions.ban_ip(database.as_ref(), &post_ip, 300_000_000) {
                    Ok(_) => {
                        str_out.push_str(&format!("Banned IP: {}\n", &post_ip));
                    },
                    Err(err) => {
                        str_out.push_str(&format!("Error: {:?}\n", err));
                    },
                }
            }

            match database.get_all_posts_by_ip(post_ip) {
                Ok(posts) => {
                    for post in &posts {
                        if dry_run {
                            str_out.push_str(&format!("Would delete post: {}\n", post.post_num()));
                        } else {
                            match actions.delete_post(
                                database.as_ref(),
                                file_rack.as_ref(),
                                post.board_id(),
                                post.post_num(),
                            ) {
                                Ok(_) => str_out
                                    .push_str(&format!("Deleted post: {}\n", post.post_num())),
                                Err(err) => str_out.push_str(&format!(
                                    "Error deleting post {}: {:?}\n",
                                    post.post_num(),
                                    err
                                )),
                            }
                        }
                    }
                },
                Err(err) => {
                    str_out.push_str(&format!("Error: {:?}\n", err));
                },
            }

            str_out
        },

        "modq" => {
            if parts.len() < 2 {
                return String::from("modq <board_id>\n");
            }

            let board_id = match parts[1].parse::<u64>() {
                Ok(id) => id,
                Err(_) => {
                    return String::from("Board ID did not parse");
                },
            };

            let site = match database.get_site() {
                Ok(site) => site,
                Err(err) => return format!("Error getting site: {:?}", err),
            };

            let mut str_out = String::new();

            let print_orig = |str_out: &mut String, orig: &site::Original| {
                str_out.push_str(&format!(
                    "==> No. {} : {} : {}\n",
                    orig.post_num(),
                    orig.poster().unwrap_or("Anonymous"),
                    orig.title().unwrap_or("<untitled>")
                ));
                str_out.push_str(&format!(
                    "\t{}/files/{}\n",
                    site.url.as_deref().unwrap_or(""),
                    orig.file_id().unwrap_or("")
                ));
                str_out.push_str(&format!("{}\n\n", orig.body()));
            };

            let print_reply = |str_out: &mut String, reply: &site::Reply| {
                str_out.push_str(&format!(
                    "==> No. {} : {}\n",
                    reply.post_num(),
                    reply.poster().unwrap_or("Anonymous")
                ));
                str_out.push_str(&format!(
                    "\t{}/files/{}\n",
                    site.url.as_deref().unwrap_or(""),
                    reply.file_id().unwrap_or("")
                ));
                str_out.push_str(&format!("{}\n\n", reply.body()));
            };

            str_out.push_str("=-=-= Originals, Unapproved =-=-=\n");
            if let Ok(orig_unapproved) =
                database.get_originals_by_approval(board_id, site::Approval::Unapproved)
            {
                orig_unapproved
                    .iter()
                    .for_each(|orig: &site::Original| print_orig(&mut str_out, orig))
            } else {
                str_out.push_str("Fetch Failed\n");
            }

            str_out.push_str("=-=-= Originals, Flagged =-=-=\n");
            if let Ok(orig_flagged) =
                database.get_originals_by_approval(board_id, site::Approval::Flagged)
            {
                orig_flagged
                    .iter()
                    .for_each(|orig: &site::Original| print_orig(&mut str_out, orig))
            } else {
                str_out.push_str("Fetch Failed\n");
            }

            str_out.push_str("=-=-= Replies, Unapproved =-=-=\n");
            if let Ok(replies_unapproved) =
                database.get_replies_by_approval(board_id, site::Approval::Unapproved)
            {
                replies_unapproved
                    .iter()
                    .for_each(|reply: &site::Reply| print_reply(&mut str_out, reply))
            } else {
                str_out.push_str("Fetch Failed\n");
            }

            str_out.push_str("=-=-= Replies, Flagged =-=-=\n");
            if let Ok(replies_flagged) =
                database.get_replies_by_approval(board_id, site::Approval::Flagged)
            {
                replies_flagged
                    .iter()
                    .for_each(|reply: &site::Reply| print_reply(&mut str_out, reply))
            } else {
                str_out.push_str("Fetch Failed\n");
            }

            str_out.push_str("-------\n");

            str_out
        },

        _ => String::from("?\n"),
    }
}
