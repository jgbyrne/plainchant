#[derive(Debug)]
pub enum Feather {
    None,
    Trip(String),
    Moderator,
    Admin,
}

pub trait Post {
    fn board_id(&self) -> u64;
    fn set_post_num(&mut self, post_num: u64);
    fn post_num(&self) -> u64;
    fn time(&self) -> u64;
    fn ip(&self) -> &str;
    fn poster(&self) -> Option<&str>;
    fn body(&self) -> &str;
    fn set_feather(&mut self, feather: Feather);
    fn feather(&self) -> &Feather;
    fn file_id(&self) -> Option<&str>;
    fn file_name(&self) -> Option<&str>;
}

#[derive(Debug)]
pub struct Original {
    pub board_id:    u64,
    pub post_num:    u64,
    pub time:        u64,
    pub ip:          String,
    pub poster:      Option<String>,
    pub body:        String,
    pub feather:     Feather,
    pub file_id:     Option<String>,
    pub file_name:   Option<String>,
    pub title:       Option<String>,
    pub bump_time:   u64,
    pub replies:     u16,
    pub img_replies: u16,
    pub pinned:      bool,
    pub archived:    bool,
}

#[derive(Debug)]
pub struct Reply {
    pub board_id:  u64,
    pub post_num:  u64,
    pub time:      u64,
    pub ip:        String,
    pub poster:    Option<String>,
    pub body:      String,
    pub feather:   Feather,
    pub file_id:   Option<String>,
    pub file_name: Option<String>,
    pub orig_num:  u64,
}

#[allow(unused)]
impl Original {
    pub fn title(&self) -> Option<&str> {
        if let Some(ref t) = self.title {
            Some(t)
        } else {
            None
        }
    }

    pub fn bump_time(&self) -> u64 {
        self.bump_time
    }

    pub fn set_bump_time(&mut self, bump_time: u64) {
        self.bump_time = bump_time;
    }

    pub fn replies(&self) -> u16 {
        self.replies
    }

    pub fn set_replies(&mut self, replies: u16) {
        self.replies = replies;
    }

    pub fn img_replies(&self) -> u16 {
        self.img_replies
    }

    pub fn set_img_replies(&mut self, img_replies: u16) {
        self.img_replies = img_replies;
    }

    pub fn pinned(&self) -> bool {
        self.pinned
    }

    pub fn set_pinned(&mut self, pinned: bool) {
        self.pinned = pinned;
    }
}

macro_rules! impl_post {
    ($($post_t:ty),+) => {
        $(impl Post for $post_t {
            fn board_id(&self) -> u64 {
                self.board_id
            }

            fn post_num(&self) -> u64 {
                self.post_num
            }

            fn set_post_num(&mut self, post_num: u64) {
                self.post_num = post_num
            }

            fn time(&self) -> u64 {
                self.time
            }

            fn ip(&self) -> &str {
                &self.ip
            }

            fn poster(&self) -> Option<&str> {
                self.poster.as_deref()
            }

            fn body(&self) -> &str {
                &self.body
            }

            fn feather(&self) -> &Feather {
                &self.feather
            }

            fn set_feather(&mut self, feather: Feather) {
                self.feather = feather
            }

            fn file_id(&self) -> Option<&str> {
                self.file_id.as_deref()
            }

            fn file_name(&self) -> Option<&str> {
                self.file_name.as_deref()
            }
        })+
    }
}

impl_post!(Original, Reply);

#[derive(Debug)]
pub struct Board {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub post_cap: u16,
    pub bump_limit: u16,
    pub next_post_num: u64,
}

#[derive(Debug)]
pub struct Catalog {
    pub board_id:  u64,
    pub time:      u64,
    pub originals: Vec<Original>,
}

#[derive(Debug)]
pub struct Site {
    pub name:        String,
    pub description: String,
    pub contact:     Option<String>,
    pub url:         Option<String>,
}

#[derive(Debug, Clone)]
pub struct Ban {
    pub id:           u64,
    pub ip:           String,
    pub time_expires: u64,
}
