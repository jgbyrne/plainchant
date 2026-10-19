## Plainchant

Plainchant is a lightweight and libre imageboard software package.

### Goals

* Practical - the software should be fully-featured and polished so that it can be used for real-world public imageboards.

* Elegant - the software should be thoughtfully engineered, robustly architected, and avoid bodged features.

* Efficient - all aspects of the software should be fast and use system resources frugally.

* Minimalist - the front-end should be minimally designed. The web markup should be handwritten and semantically meaningful. The site should not use JavaScript.

* Modern - minimal does not mean antiquated. Modern technologies should be employed throughout the project.

* Familiar - the software should employ the conventions of existing imageboards to ensure familiarity

### Non-goals

* Compatibility - the software should not compromise on the goals above for the sake of supporting obsolete environments or to perfectly copy existing imageboards

* Webscale - the software is intended for small and medium size communities. It should scale respectably on a single node but should not attempt to support distributed or sharded deployments.

### Technologies

Plainchant is written in Rust. It uses the Axum web framework and the SQLite database. It is packaged for deployment as a Nix flake.

-----

## Building Plainchant

The recommended way of building this project is with `nix`. You should make sure that flakes are enabled in your `nix` distribution. Then you can do `nix build .#debug` and `nix build .#release`. It is also possible to build `plainchant` as a conventional rust project with `cargo` if you prefer.

## Running Plainchant

The following steps describe a convenient way of setting up Plainchant for local development. Note that all directories specified here are recommendations and that others may be used, provided the config file `plainchant.toml` is updated appropriately.

1. Create a directory `/etc/plainchant`, and into it copy from this repository the file `example/plainchant.toml` and the folders `templates` and `static`. † 

2. Create a directory `/var/lib/plainchant` and within it the subdirectory `fsfr`.

3. Ensure that the user that you intend to run the server has read access to `/etc/plainchant/` and read-write access to `/var/lib/plainchant/`

4. The next thing to do is to initialise the database at the path specified in the config file. With `nix`, this is `nix run .# -- <config_path> --init-only`, otherwise, you can use cargo or just run the `plainchant` binary directly.

5. Using a tool of your choice, add each board that you wish to serve into the `Boards` table of the sqlite3 database. The schema is (`BoardId`, `Url`, `Title`, `PostCap`, `BumpLimit`, `NextPostNum`, `ArchiveCap`). For example:

    `INSERT INTO Boards VALUES (1001, 'mu', 'Music', 20, 100, 10000, 10);`

6. Using a tool of your choice, update the singleton row in the `Site` table of the database with a site name and description of your choice. For example:

    `REPLACE INTO Site VALUES (1, 'sandcastlechan', 'An imageboard all about sandcastles.', 'webmaster@sandcastlechan.net', 'https://sandcastlechan.net');`

7.  You can now run `plainchant`. By default it runs on `http://localhost:8088`.

† *You will probably find it useful to symlink these directories to your local copy of the repository for easier hacking*
