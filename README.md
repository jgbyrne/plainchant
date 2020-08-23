## Plainchant

![Demo Site Screenshot](https://github.com/jgbyrne/plainchant/blob/master/demo/screenshot.png)

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

*Disclaimer: This is a technical project intended to replicate to some extent the functionality and aesthetics of internet imageboard websites. There are many such sites, of which some, including 4chan and 8chan, have gained notoriety for distasteful content. The existence of this project does not imply my agreement, tacit or overt, with anything shared on any such site. Obviously.*
