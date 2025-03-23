## Plainchant

Plainchant is a lightweight and libre imageboard software package.

### Goals

* Practical - the software should be fully-featured and polished such that it can be used for real-world public imageboards.

* Robust - the software should be thoughtfully engineered and take the right choices, not the easy ones.

* Efficient - all aspects of the system should be fast and use system resources frugally.

* Minimalist - the front-end should be minimally designed. The web markup should be handwritten, thoughtful, and semantically meaningful. The site should not use JavaScript, and be viewable even in a text-mode browser.

* Modern - minimalism does not mean stuck in the past. Elegant and modern technologies should be employed throughout the project.

* Familiar - the software should employ the conventions of existing imageboards to ensure familiarity

### Non-goals

* Compatibility - the software should not compromise on elegance for the sake of supporting obsolete environments or to perfectly copy existing imageboards

* Webscale - the software is intended for small and medium size communities. It should scale respectably, but is not designed to effortlessly host the next Reddit. 

### Technologies

Plainchant is written in Rust. It uses the Axum web framework and the SQLite database. 

-----

## Running Plainchant

The following steps describe a convenient way of setting up Plainchant for local development. Note that all directories specified here are recommendations and that others may be used, provided the config file `plainchant.toml` is updated appropriately.

1. Create a directory `/etc/plainchant`, and into it copy from this repository the file `example/plainchant.toml` and the folders `templates` and `static`. † 

2. Create a directory `/var/lib/plainchant` and within it the subdirectory `fsfr`.

3. Ensure that the user that you intend to run the server has read access to `/etc/plainchant/` and read-write access to `/var/lib/plainchant/`

4. To create the database at `/var/lib/plainchant/db.sqlite3`, run `plainchant`, either with `cargo run` or by invoking the binary directly. You need provide just one argument, the path to the site config file - if you have exactly followed the directions above, that's `/etc/plainchant/plainchant.toml`.

5. Using a tool of your choice, add each board that you wish to serve into the `Boards` table of the sqlite3 database. The schema is (`BoardId`, `Url`, `Title`, `PostCap`, `BumpLimit`, `NextPostNum`). For example:

    `INSERT INTO Boards VALUES (1234, 'mu', 'Music', 20, 100, 10000);`

6. Using a tool of your choice, update the singleton row in the `Site` table of the database with a site name and description of your choice. For example:

    `REPLACE INTO Site VALUES (1, 'sandcastlechan', 'An imageboard all about sandcastles.', 'webmaster@sandcastlechan.net');`

7.  You can now run `plainchant`. By default it runs on `localhost:8088`.

† *You may find it useful to symlink these directories to your local copy of the repository for ease-of-hacking* 
