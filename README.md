## Plainchant

![Demo Screenshot](https://github.com/jgbyrne/plainchant/blob/master/demo/screenshot.png)

Plainchant is a lightweight and libre imageboard software package.

### Goals

* Practical - the software should be fully-featured and polished such that it can be used for real-world public imageboards.

* Robust - the software should be thoughtfully engineered and take the right choices, not the easy ones.

* Efficient - all aspects of the system should be fast and use system resources frugally.

* Minimalist - the front-end should be minimally designed. The web markup should be handwritten, thoughtful, and semantically meaningful. The site should be fully usable without javascript, and viewable even in a text-mode browser.

* Modern - minimalism does not mean stuck in the past. Elegant and modern technologies should be employed throughout the project.

* Familiar - the software should employ the conventions of existing imageboards to ensure familiarity

### Non-goals

* Compatibility - the software should not compromise on elegance for the sake of supporting obsolete environments or to perfectly copy existing imageboards

* Webscale - the software is intended for small and medium size communities. It should scale respectably, but is not designed to effortlessly host the next Reddit. 

### Technologies

Plainchant is implemented in Rust. Web functionality is provided by Warp, while templating is achieved with an inbuilt engine. The intention is to support Postgres as a backend database.

-----

## Running Plainchant

**Note 2022-08-11: the following is outdated and will not work on master. Checkout legacy if you wish to deploy with these instructions**

Plainchant is alpha software and should not be used for any serious purpose. However the interested reader may use the following instructions to try it out for themselves. Note that all directories specified here are simply recommendations and that others may be used, provided the config file `plainchant.toml` is updated appropriately.

1. Create a directory `/etc/plainchant`, and into it copy from this repository the file `demo/plainchant.toml` and the folders `templates` and `static`. † 

2. Create a directory `/var/lib/plainchant` and within it two subdirectories `fsdb` and `fsfr`.

3. Create a subdirectory `/rack` within `/var/lib/plainchant/fsfr`.

4. Create a file `boards` within `/var/lib/plainchant/fsdb`

5. For each board that you wish to serve, add a line to the file `boards` of the form `<board_id>,<slug>,<desc>,<post_ctr>,<post_cap>,<bump_limit>` - for example `1234,mu,Music,10000,20,100`.

6. For each board that you wish to serve, create a directory `<board_id>` within `/var/lib/plainchant/fsdb`

7. Ensure that the user that you intend to run the server has read access to `/etc/plainchant/` and read-write access to `/var/lib/plainchant/`

† *You may find it useful to symlink these directories to your local copy of the repository for ease-of-hacking* 

You may now run `plainchant`, either with `cargo run` or by invoking the binary directly. You need provide just one argument, the path to the site config file - if you have exactly followed the directions above, that's `/etc/plainchant/plainchant.toml`. By default it runs on `localhost:8088`.

-----

*Disclaimer: This is a technical project intended to replicate to some extent the functionality and aesthetics of internet imageboard websites. There are many such sites, of which some, including 4chan and 8chan, have gained notoriety for distasteful content. The existence of this project does not imply my agreement, tacit or overt, with anything shared on any such site. Obviously.*
