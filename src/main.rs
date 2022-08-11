mod site;
mod util;

mod db;
mod fr;

mod actions;
mod format;
mod fsdb;
mod fsfr;
mod pages;
mod server;
mod sqlite3db;
mod template;

use std::convert::TryInto;
use std::env;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::process::exit;

use toml::Value;

fn init_die(msg: &str) -> ! {
    eprintln!("Initialisation Error: {}", msg);
    exit(9);
}

pub struct Config {
    addr:          SocketAddr,
    templates_dir: PathBuf,
    static_dir:    PathBuf,
}

fn val<'v_out, 'v_in: 'v_out>(v: &'v_in Value, k: &str) -> &'v_out Value {
    v.get(k)
     .unwrap_or_else(|| init_die(&format!("Could not get config key: {}", k)))
}

fn main() {
    let mut args = std::env::args();
    let _bin = args.next();

    let conf_path = fs::canonicalize(args.next().unwrap_or(String::from("./plainchant.toml")))
                            .unwrap_or_else(|_|
                                init_die("Config file does not exist."));

    let conf_string = fs::read_to_string(&conf_path).unwrap_or_else(|_| {
                                                        init_die("Could not read from config file.")
                                                    });

    let conf_data =
        conf_string.parse::<Value>()
                   .unwrap_or_else(|_| init_die("Could not parse config file as toml."));

    let conf_dir = conf_path.parent()
                            .unwrap_or_else(|| init_die("Could not get config file directory."));

    env::set_current_dir(conf_dir).unwrap_or_else(|_| {
                                      eprintln!("Could not set current directory");
                                      exit(1);
                                  });

    let ip = String::from(val(val(&conf_data, "site"), "ip").as_str()
                                                            .unwrap_or_else(|| {
                                                                init_die("site.ip is not a string")
                                                            }));

    let port: u16 = val(val(&conf_data, "site"), "port").as_integer()
                                                        .unwrap_or_else(|| init_die("site.port is not an integer"))
                                                        .try_into()
                                                        .unwrap_or_else(|_| init_die("site.port is not a sensibly sized positive integer"));
    let addr =
        (ip.parse::<IpAddr>()
           .unwrap_or_else(|_| init_die("site.ip could not be understood as an IP Address")),
         port);

    let addr: SocketAddr =
        addr.try_into()
            .unwrap_or_else(|_| init_die("site.ip:site.port is not a valid address"));

    let assets = PathBuf::from(val(val(&conf_data, "site"), "assets").as_str().unwrap_or_else(|| init_die("site.assets path is not a string")));
    let templates_dir = fs::canonicalize(assets.join("templates")).unwrap_or_else(|_| init_die("Could not comprehend templates path"));
    let static_dir =
        fs::canonicalize(assets.join("static")).unwrap_or_else(|_| {
                                                   init_die("Could not comprehend static path")
                                               });

    let config = Config { addr,
                          templates_dir,
                          static_dir };

    // Load database - this needs to be db::Database
    let db = if let Some(path) = val(val(val(&conf_data, "db"), "sqlite"), "path").as_str() {
        match fs::canonicalize(path) {
            Ok(path) => {
                sqlite3db::Sqlite3Database::from_path(PathBuf::from(path)).unwrap_or_else(|err| {
                                                                              err.die()
                                                                          })
            },
            Err(_) => init_die("Could not comprehend sqlite3db path"),
        }
    } else {
        init_die("No database specified in config")
    };

    // Load file rack - this needs to be fr::FileRack
    let fr = if let Some(path) = val(val(val(&conf_data, "fr"), "fs"), "path").as_str() {
        match fs::canonicalize(path) {
            Ok(path) => fsfr::FSFileRack::from_dir(&path.as_path()).unwrap_or_else(|err| err.die()),
            Err(_) => init_die("Could not comprehend fsfr path"),
        }
    } else {
        init_die("No file rack specified in config")
    };

    // Load templates from template files
    let templates = pages::SiteTemplates {
        catalog_tmpl: template::Template::from_file(config.templates_dir.join("catalog.html.tmpl").as_path()).unwrap_or_else(|err| err.die()),
        thread_tmpl: template::Template::from_file(config.templates_dir.join("thread.html.tmpl").as_path()).unwrap_or_else(|err| err.die()),
        create_tmpl: template::Template::from_file(config.templates_dir.join("create.html.tmpl").as_path()).unwrap_or_else(|err| err.die()),
    };

    // Create structs for pages and actions
    let pages = pages::Pages::new(&db, templates, 1).unwrap_or_else(|err| err.die());
    let actions = actions::Actions::new();

    // Serve the site using the pages, actions, and database
    server::serve(config, pages, actions, db, fr);
}
