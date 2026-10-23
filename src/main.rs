mod site;
mod util;

mod db;
mod fr;

mod actions;
mod api;
mod console;
mod format;
mod fsfr;
mod pages;
mod server;
mod sqlite3db;
mod state;
mod template;

use crate::db::Database;

use std::env;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::exit;

use serde::Deserialize;

fn is_true() -> bool {
    true
}

fn is_false() -> bool {
    false
}

#[derive(Deserialize)]
struct ConfigSite {
    addr: SocketAddr,
    assets: PathBuf,
    #[serde(default = "is_true")]
    approve_threads_by_default: bool,
    #[serde(default = "is_true")]
    approve_replies_by_default: bool,
    #[serde(default = "is_false")]
    whitelist_domains: bool,
}

#[derive(Deserialize)]
struct ConfigConsole {
    access_key: Option<String>,
}

#[derive(Deserialize)]
struct ConfigSqlite {
    path: PathBuf,
}

#[derive(Deserialize)]
struct ConfigDb {
    sqlite: ConfigSqlite,
}

#[derive(Deserialize)]
struct ConfigFsfr {
    path: PathBuf,
}

#[derive(Deserialize)]
struct ConfigFr {
    fs: ConfigFsfr,
}

#[derive(Deserialize)]
struct ConfigFile {
    site:    ConfigSite,
    console: ConfigConsole,
    db:      ConfigDb,
    fr:      ConfigFr,
}

fn init_die(msg: &str) -> ! {
    eprintln!("Initialisation Error: {}", msg);
    exit(9);
}

pub struct Config {
    addr: SocketAddr,
    templates_dir: PathBuf,
    static_dir: PathBuf,
    approve_threads_by_default: bool,
    approve_replies_by_default: bool,
    whitelist_domains: bool,
    access_key: Option<String>,
}

fn main() {
    let mut args = std::env::args();
    let _bin = args.next();

    let conf_path_raw = args
        .next()
        .unwrap_or_else(|| init_die("plainchant <config_path> [--init-only]"));

    let init_only = match args.next().as_deref() {
        Some("--init-only") => true,
        Some(_) => init_die("Unrecognised option"),
        _ => false,
    };

    let conf_path =
        fs::canonicalize(conf_path_raw).unwrap_or_else(|_| init_die("Config file does not exist."));

    let conf_string = fs::read_to_string(&conf_path)
        .unwrap_or_else(|_| init_die("Could not read from config file."));

    let raw_config: ConfigFile = toml::from_str(&conf_string)
        .unwrap_or_else(|err| init_die(&format!("Could not parse config file.\n{}", err)));

    let conf_dir = conf_path
        .parent()
        .unwrap_or_else(|| init_die("Could not get config file directory."));

    env::set_current_dir(conf_dir).unwrap_or_else(|_| {
        eprintln!("Could not set current directory");
        exit(1);
    });

    let templates_dir = fs::canonicalize(raw_config.site.assets.join("templates"))
        .unwrap_or_else(|_| init_die("Could not comprehend templates path"));

    let static_dir = fs::canonicalize(raw_config.site.assets.join("static"))
        .unwrap_or_else(|_| init_die("Could not comprehend static path"));

    let config = Config {
        addr: raw_config.site.addr,
        templates_dir,
        static_dir,
        approve_threads_by_default: raw_config.site.approve_threads_by_default,
        approve_replies_by_default: raw_config.site.approve_replies_by_default,
        whitelist_domains: raw_config.site.whitelist_domains,
        access_key: raw_config.console.access_key,
    };

    // Load database - this needs to be db::Database
    let parent = raw_config
        .db
        .sqlite
        .path
        .parent()
        .unwrap_or_else(|| init_die("Database path has no parent"));

    let file_name = raw_config
        .db
        .sqlite
        .path
        .file_name()
        .unwrap_or_else(|| init_die("Database path has no file name"));

    let db = match fs::canonicalize(parent) {
        Ok(pp) => sqlite3db::Sqlite3Database::from_path(pp.join(file_name))
            .unwrap_or_else(|err| err.die()),
        Err(_) => init_die("Could not comprehend sqlite3db path"),
    };

    // Load file rack - this needs to be fr::FileRack
    let fr = match fs::canonicalize(raw_config.fr.fs.path) {
        Ok(path) => fsfr::FSFileRack::from_dir(&path).unwrap_or_else(|err| err.die()),
        Err(_) => init_die("Could not comprehend fsfr path"),
    };

    // Load templates from template files
    let templates = pages::SiteTemplates {
        homepage_tmpl: template::Template::from_file(
            config.templates_dir.join("homepage.html.tmpl").as_path(),
        )
        .unwrap_or_else(|err| err.die()),
        catalog_tmpl:  template::Template::from_file(
            config.templates_dir.join("catalog.html.tmpl").as_path(),
        )
        .unwrap_or_else(|err| err.die()),
        archive_tmpl:  template::Template::from_file(
            config.templates_dir.join("archive.html.tmpl").as_path(),
        )
        .unwrap_or_else(|err| err.die()),
        thread_tmpl:   template::Template::from_file(
            config.templates_dir.join("thread.html.tmpl").as_path(),
        )
        .unwrap_or_else(|err| err.die()),
        create_tmpl:   template::Template::from_file(
            config.templates_dir.join("create.html.tmpl").as_path(),
        )
        .unwrap_or_else(|err| err.die()),
    };

    // Create structs for pages and actions
    let site = db
        .get_site()
        .unwrap_or_else(|_| init_die("No site configured in database"));

    let pages = pages::Pages::new(site, templates, 1).unwrap_or_else(|err| err.die());
    let actions = actions::Actions::new(&db).unwrap_or_else(|err| err.die());

    if init_only {
        eprintln!("Initialisation complete");
        exit(0);
    }

    // Serve the site using the pages, actions, and database
    server::serve(config, pages, actions, db, fr);
}
