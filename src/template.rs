use crate::util;
use std::fs;
use std::mem;
use std::collections::HashMap;

pub struct Data {
    values: HashMap<String, String>,
    collections: HashMap<String, Vec<String>>,
}

impl Data {
    pub fn new(values: HashMap<String, String>, collections: HashMap<String, Vec<String>>) -> Data {
        Data {
            values,
            collections,
        }
    }
}

#[derive(Debug)]
pub enum Chunk {
    Fragment(String),
    Placeholder(String, Option<String>),
    Control(String),
}

#[derive(Debug)]
pub struct Template {
    chunks: Vec<Chunk>,
}

pub fn static_err(msg: &'static str) -> util::PlainchantErr {
    util::PlainchantErr { 
        origin: util::ErrOrigin::Template,
        msg: msg.to_string()
    }
}

impl Template {
    pub fn render(&self, data: &Data) -> String {
        let empty_str = String::from("");
        let mut buf = String::new();
        let mut cptr = 0;
        let mut ptrs: HashMap<String, usize> = HashMap::new();
        let mut ctrs: HashMap<String, usize> = HashMap::new();
        loop {
            if cptr >= self.chunks.len() {
                break;
            }
            let chunk = &self.chunks[cptr];
            match chunk {
                Chunk::Fragment(s) => buf.push_str(&s),
                Chunk::Placeholder(name, obj) => {
                    match obj {
                        Some(obj_name) => {
                            if let Some(obj_ctr) = ctrs.get(obj_name) {
                                if let Some(obj_col) = data.collections.get(obj_name) {
                                    let obj = &obj_col[*obj_ctr];
                                    let mut valpath = String::from(obj_name);
                                    valpath.push('.');
                                    valpath.push_str(&obj);
                                    valpath.push('.');
                                    valpath.push_str(name);
                                    buf.push_str(data.values.get(&valpath).unwrap_or(&empty_str));
                                };
                            }
                        },
                        None => {
                            match name.as_str() {
                                "$TIME" => buf.push_str(&util::timestamp().to_string()),
                                "$PLAINCHANT" => buf.push_str(&format!("Plainchant v{}", env!("CARGO_PKG_VERSION"))),
                                _ => buf.push_str(data.values.get(name).unwrap_or(&empty_str)),
                            }
                        },
                    }
                },
                Chunk::Control(obj) => {
                    match ptrs.get(obj) {
                        Some(start_ptr) => {
                            if *start_ptr != cptr {
                                let mut ctr = *ctrs.get(obj).unwrap();
                                ctr += 1;
                                if ctr == data.collections.get(obj).unwrap().len() {
                                    ptrs.remove(obj);
                                    ctrs.remove(obj);
                                }
                                else {
                                    ctrs.insert(String::from(obj), ctr);
                                    cptr = *start_ptr;
                                }
                            }
                        },
                        None =>  {
                            ptrs.insert(String::from(obj), cptr);
                            ctrs.insert(String::from(obj), 0);
                        },
                    }
                },
            }
            cptr += 1;
        }
        buf
    }

    pub fn from_string(string: String) -> Result<Template, util::PlainchantErr> {
        let mut chunks = vec![];
        let mut buf = String::new();
        let mut state = '+';
        for c in string.chars() {
            match state {
                '+' => {
                    match c {
                        '{' => state = '{',
                        _   => buf.push(c),
                    }
                },
                '{' => {
                    match c {
                        '{' => state = '!',
                        '%' => state = '?',
                        _   => { buf.push('{'); buf.push(c); state = '+'; },
                    }
                    if state != '+' {
                       let frag = mem::replace(&mut buf, String::new());
                       chunks.push(Chunk::Fragment(frag));
                    }
                },
                '!' => {
                    match c {
                        '}' => {
                            let raw = mem::replace(&mut buf, String::new());
                            let split = raw.split(".").collect::<Vec<&str>>();
                            match split.len() {
                                1 => chunks.push(Chunk::Placeholder(raw, None)),
                                2 => chunks.push(Chunk::Placeholder(
                                        split[1].to_string(),
                                        Some(split[0].to_string()))),
                                _ => return Err(static_err("Bad syntax")),
                            }
                            state = '}';
                        },
                        _ => buf.push(c),
                    }
                },
                '?' => {
                    match c {
                        '%' => {
                            let raw = mem::replace(&mut buf, String::new());
                            chunks.push(Chunk::Control(raw));
                            state = '}';
                        },
                        _ => buf.push(c),
                    }
                },
                '}' => {
                    if c != '}' {
                        return Err(static_err("Invalid syntax"));
                    }
                    else {
                        state = '+';
                    }
                },
                sc@_ => {println!("Entered invalid state {}", sc); panic!()},
            }
        }
        if buf.len() > 0 {
            chunks.push(Chunk::Fragment(buf));
        }
        Ok(Template { chunks })
    }

    pub fn from_file(path: &str) -> Result<Template, util::PlainchantErr> {
        match fs::read_to_string(path) {
            Ok(s) => Template::from_string(s),
            Err(_) => Err(static_err("Could not read from template file")),
        }
    }
}
