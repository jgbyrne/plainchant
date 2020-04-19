pub trait Post {
    fn board_id(&self) -> u64;
    fn post_num(&self) -> u64;
    fn time(&self) -> u64;
    fn ip(&self) -> &str;
    fn body(&self) -> &str;
    fn poster(&self) -> Option<&str>;
    fn file_id(&self) -> Option<&str>;
    fn file_name(&self) -> Option<&str>;
}

#[derive(Debug)]
pub struct Original {
    board_id  : u64,
    post_num  : u64,
    time      : u64,
    ip        : String,
    body      : String,
    poster    : Option<String>,
    file_id   : Option<String>,
    file_name : Option<String>,
    title       : Option<String>,
    replies     : u16,
    img_replies : u16,
}

impl Original {
    pub fn new(board_id: u64, post_num: u64, time: u64, ip: String, body: String,
           poster: Option<String>, file_id: Option<String>, file_name: Option<String>,
           title: Option<String>, replies: u16, img_replies: u16 ) -> Original {
        Original {
            board_id,
            post_num,
            time,
            ip,
            body,
            poster,
            file_id,
            file_name,
            title,
            replies,
            img_replies,
        }
    }
}

#[derive(Debug)]
pub struct Reply {
    board_id  : u64,
    post_num  : u64,
    time      : u64,
    ip        : String,
    body      : String,
    poster    : Option<String>,
    file_id   : Option<String>,
    file_name : Option<String>,
    orig_num    : u64,
}

impl Reply {
    pub fn new(board_id: u64, post_num: u64, time: u64, ip: String, body: String,
           poster: Option<String>, file_id: Option<String>, file_name: Option<String>,
           orig_num: u64 ) -> Reply {
        Reply {
            board_id,
            post_num,
            time,
            ip,
            body,
            poster,
            file_id,
            file_name,
            orig_num,
        }
    }
}

impl Original {
    pub fn title(&self) -> Option<&str> {
        if let Some(ref t) = self.title {
            Some(t)
        }
        else {
            None
        }
    }
}

impl Reply {
    pub fn orig_num(&self) -> u64 {
        self.orig_num
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

            fn time(&self) -> u64 {
                self.time
            }

            fn ip(&self) -> &str {
                &self.ip
            }

            fn body(&self) -> &str {
                &self.body
            }

            fn poster(&self) -> Option<&str> {
                if let Some(ref p) = self.poster {
                    Some(p)
                }
                else {
                    None
                }
            }

            fn file_id(&self) -> Option<&str> {
                if let Some(ref f_id) = self.file_id {
                    Some(f_id)
                }
                else {
                    None
                }
            }
            
            fn file_name(&self) -> Option<&str> {
                if let Some(ref f_name) = self.file_name {
                    Some(f_name)
                }
                else {
                    None
                }           
            }
        })+
    }
}

impl_post!(Original, Reply);

#[derive(Debug)]
pub struct Board {
    pub id    : u64,
    pub url   : String,
    pub title : String,
}

pub struct Catalog {
    pub board_id : u64,
    pub time     : u64,
    pub originals: Vec<Original>,
}
