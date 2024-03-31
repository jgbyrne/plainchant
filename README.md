## Plainchant

<p align="center">
   <img src="https://github.com/jgbyrne/plainchant/blob/master/demo/screenshot.png" width="500px" title="Demo Screenshot"></img>
</p>

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

Plainchant is implemented in Rust. Web functionality is provided by Axum, while templating is achieved with an inbuilt engine. SQLite is used for backend data storage. 

-----

## Running Plainchant

Plainchant is alpha software and should not be used for any serious purpose. However the interested reader may use the following instructions to try it out for themselves. Note that all directories specified here are simply recommendations and that others may be used, provided the config file `plainchant.toml` is updated appropriately.

1. Create a directory `/etc/plainchant`, and into it copy from this repository the file `demo/plainchant.toml` and the folders `templates` and `static`. † 

2. Create a directory `/var/lib/plainchant` and within it the subdirectory `fsfr`.

3. Ensure that the user that you intend to run the server has read access to `/etc/plainchant/` and read-write access to `/var/lib/plainchant/`

4. To create the database at `/var/lib/plainchant/db.sqlite3`, run `plainchant`, either with `cargo run` or by invoking the binary directly. You need provide just one argument, the path to the site config file - if you have exactly followed the directions above, that's `/etc/plainchant/plainchant.toml`.

5. Using a tool of your choice, add each board that you wish to serve into the `Boards` table of the sqlite3 database. The schema is (`BoardId`, `Url`, `Title`, `PostCap`, `BumpLimit`, `NextPostNum`). For example:

    `INSERT INTO Boards VALUES (1234, 'mu', 'Music', 20, 100, 10000);`

6. Using a tool of your choice, update the singleton row in the `Site` table of the database with a site name and description of your choice. For example:

    `REPLACE INTO Site VALUES (1, "sandcastlechan", "An imageboard all about sandcastles.");`

7.  You can now run `plainchant` in earnest. By default it runs on `localhost:8088`.

† *You may find it useful to symlink these directories to your local copy of the repository for ease-of-hacking* 

-----

*Disclaimer: This is a technical project intended to replicate to some extent the functionality and aesthetics of internet imageboard websites. There are many such sites, of which some, including 4chan and 8chan, have gained notoriety for distasteful content. The existence of this project does not imply my agreement, tacit or overt, with anything shared on any such site. Obviously.*
