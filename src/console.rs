use crate::actions;
use crate::db;
use crate::fr;
use crate::site;
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
                return String::from("post (show|rm|mod|admin|nocap) <board_id> <post_num>\n");
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

        "wipe" => {
            if parts.len() < 2 {
                return String::from("wipe <ip>\n");
            }

            match actions.delete_all_posts_by_ip(
                database.as_ref(),
                file_rack.as_ref(),
                parts[1].trim().to_string(),
            ) {
                Ok(num_posts) => format!("Deleted {} posts\n", num_posts),
                Err(err) => format!("Error: {:?}\n", err),
            }
        },

        _ => String::from("?\n"),
    }
}
