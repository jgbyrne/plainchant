mod site;
mod util;

mod db;

mod fr;

mod actions;
mod fsdb;
mod fsfr;
mod pages;
mod server;
mod template;

use std::env;
use std::fs;
use std::path::Path;
use std::process::exit;

use toml::Value;

fn init_die(msg: &str) -> ! {
    eprintln!("Initialisation Error: {}", msg);
    exit(9);
}

fn main() {
    let mut args = std::env::args();
    let bin = args.next();

    let conf_path = fs::canonicalize(args.next().unwrap_or(String::from("./plainchant.toml")))
                            .unwrap_or_else(|_|
                                init_die("Config file does not exist."));

    let mut conf_string = fs::read_to_string(&conf_path)
                                .unwrap_or_else(|_|
                                    init_die("Could not read from config file."));

    let mut config = conf_string.parse::<Value>()
                                .unwrap_or_else(|_|
                                    init_die("Could not parse config file as toml."));

    let conf_dir = conf_path.parent()
                            .unwrap_or_else(||
                                init_die("Could not get config file directory."));

    env::set_current_dir(conf_dir)
            .unwrap_or_else(|_| {
                eprintln!("Could not set current directory");
                exit(1);
            });

    // Load database - this needs to be db::Database
    let db = if let Some(path) = config["db"]["fs"]["path"].as_str() {
        match fs::canonicalize(path) {
            Ok(path) => fsdb::FSDatabase::from_root(path.to_str().unwrap_or("")).unwrap_or_else(|err| err.die()),
            Err(_)   => init_die("Could not comprehend fsdb path"),
        }
    }
    else {
        init_die("No database specified in config")
    };

    // Load file rack - this needs to be fr::FileRack
    let fr = if let Some(path) = config["fr"]["fs"]["path"].as_str() {
        match fs::canonicalize(path) {
            Ok(path) => fsfr::FSFileRack::from_dir(path.to_str().unwrap_or("")).unwrap_or_else(|err| err.die()),
            Err(_)   => init_die("Could not comprehend fsfr path"),
        }
    }
    else {
        init_die("No file rack specified in config")
    };

    let tmpl_path = fs::canonicalize(Path::new(config["site"]["templates"].as_str().unwrap())).unwrap_or_else(|_| init_die("Could not comprehend templates path"));

    // Load templates from template files
    let templates = pages::SiteTemplates {
        catalog_tmpl: template::Template::from_file(tmpl_path.join("catalog.html.tmpl").to_str().unwrap()).unwrap(),
        thread_tmpl: template::Template::from_file(tmpl_path.join("thread.html.tmpl").to_str().unwrap()).unwrap(),
        create_tmpl: template::Template::from_file(tmpl_path.join("create.html.tmpl").to_str().unwrap()).unwrap(),
    };

    // Create structs for pages and actions
    let pages = pages::Pages::new(&db, templates, 1).unwrap();
    let actions = actions::Actions::new();

    // Serve the site using the pages, actions, and database
    server::serve(pages, actions, db, fr, [0, 0, 0, 0], 8088);
}
