use crate::util;
use std::collections::HashMap;
use std::fs;
use std::mem;

pub struct Data {
    values:      HashMap<String, String>,
    flags:       HashMap<String, bool>,
    collections: HashMap<String, Vec<String>>,
}

impl Data {
    pub fn new(values: HashMap<String, String>, flags: HashMap<String, bool>, collections: HashMap<String, Vec<String>>) -> Data {
        Data { values,
               flags,
               collections }
    }
}

#[derive(Debug)]
pub enum Chunk {
    Fragment(String),
    Placeholder(String, Option<String>),
    Condition(String, Option<String>),
    Control(String),
}

#[derive(Debug)]
pub struct Template {
    chunks: Vec<Chunk>,
}

pub fn static_err(msg: &'static str) -> util::PlainchantErr {
    util::PlainchantErr { origin: util::ErrOrigin::Template,
                          msg:    msg.to_string(), }
}

impl Template {
    pub fn render(&self, data: &Data) -> String {
        let empty_str = String::from("");
        let mut buf = String::new();
        let mut cptr = 0;
        let mut ptrs: HashMap<String, usize> = HashMap::new();
        let mut ctrs: HashMap<String, usize> = HashMap::new();
        let mut skip: Option<String> = None;
        loop {
            if cptr >= self.chunks.len() {
                break;
            }
            let chunk = &self.chunks[cptr];
            match chunk {
                Chunk::Fragment(s) => {
                    if skip.is_none() {
                        buf.push_str(&s)
                    }
                },
                Chunk::Placeholder(name, obj) => {
                    if skip.is_none() {
                        match obj {
                            Some(obj_name) => {
                                if let Some(obj_ctr) = ctrs.get(obj_name) {
                                    if let Some(obj_col) = data.collections.get(obj_name) {
                                        let obj_id = &obj_col[*obj_ctr];
                                        let mut valpath = String::from(obj_name);
                                        valpath.push('.');
                                        valpath.push_str(&obj_id);
                                        valpath.push('.');
                                        valpath.push_str(name);
                                        buf.push_str(data.values
                                                         .get(&valpath)
                                                         .unwrap_or(&empty_str));
                                    };
                                }
                            },
                            None => match name.as_str() {
                                "$TIME" => buf.push_str(&util::timestamp().to_string()),
                                "$PLAINCHANT" => buf.push_str(&format!("Plainchant v{}",
                                                                       env!("CARGO_PKG_VERSION"))),
                                _ => buf.push_str(data.values.get(name).unwrap_or(&empty_str)),
                            },
                        }
                    }
                },
                Chunk::Condition(ref name, ref obj) => {
                    if let Some(ref s) = skip {
                        if *s == format!("{}.{}", name, obj.as_ref().unwrap_or(&empty_str)) {
                            skip = None;  
                        }
                    } else {
                        let flag = match obj {
                            Some(obj_name) => {
                                if let Some(obj_ctr) = ctrs.get(obj_name) {
                                    if let Some(obj_col) = data.collections.get(obj_name) {
                                        let obj_id = &obj_col[*obj_ctr];
                                        let mut valpath = String::from(obj_name);
                                        valpath.push('.');
                                        valpath.push_str(&obj_id);
                                        valpath.push('.');
                                        valpath.push_str(name);
                                        *data.flags.get(&valpath).unwrap_or(&false)
                                    } else { false }
                                } else { false }
                            },
                            None => {
                                *data.flags.get(name).unwrap_or(&false)
                            }
                        };
                        if !flag {
                            skip = Some(format!("{}.{}", name, obj.as_ref().unwrap_or(&empty_str)));
                        }
                    }
                },
                Chunk::Control(obj) => match ptrs.get(obj) {
                    Some(start_ptr) => {
                        if skip.is_none() {
                            if *start_ptr != cptr {
                                let mut ctr = *ctrs.get(obj).unwrap();
                                ctr += 1;
                                if ctr == data.collections.get(obj).unwrap().len() {
                                    ptrs.remove(obj);
                                    ctrs.remove(obj);
                                } else {
                                    ctrs.insert(String::from(obj), ctr);
                                    cptr = *start_ptr;
                                }
                            }
                        }
                    },
                    None => {
                        if let Some(ref s) = skip {
                            if s == obj {
                                skip = None;
                            }
                        } else {
                            match data.collections.get(obj) {
                                Some(col) => {
                                    if col.len() == 0 {
                                        skip = Some(obj.clone());
                                    } else {
                                        ptrs.insert(String::from(obj), cptr);
                                        ctrs.insert(String::from(obj), 0);
                                    }
                                },
                                None => {},
                            }
                        }
                    },
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
                '+' => match c {
                    '{' => state = '{',
                    _ => buf.push(c),
                },
                '{' => {
                    match c {
                        '{' => state = '$',
                        ':' => state = '?',
                        '%' => state = '!',
                        _ => {
                            buf.push('{');
                            buf.push(c);
                            state = '+';
                        },
                    }
                    if state != '+' {
                        let frag = mem::replace(&mut buf, String::new());
                        chunks.push(Chunk::Fragment(frag));
                    }
                },
                '$' => match c {
                    '}' => {
                        let raw = mem::replace(&mut buf, String::new());
                        let split = raw.split(".").collect::<Vec<&str>>();
                        match split.len() {
                            1 => chunks.push(Chunk::Placeholder(raw, None)),
                            2 => chunks.push(Chunk::Placeholder(split[1].to_string(),
                                                                Some(split[0].to_string()))),
                            _ => return Err(static_err("Bad syntax")),
                        }
                        state = '}';
                    },
                    _ => buf.push(c),
                },
                '?' => match c {
                    ':' => {
                        let raw = mem::replace(&mut buf, String::new());
                        let split = raw.split(".").collect::<Vec<&str>>();
                        match split.len() {
                            1 => chunks.push(Chunk::Condition(raw, None)),
                            2 => chunks.push(Chunk::Condition(split[1].to_string(),
                                                              Some(split[0].to_string()))),
                            _ => return Err(static_err("Bad syntax")),
                        }
                        state = '}';
                    },
                    _ => buf.push(c),
                },
                '!' => match c {
                    '%' => {
                        let raw = mem::replace(&mut buf, String::new());
                        chunks.push(Chunk::Control(raw));
                        state = '}';
                    },
                    _ => buf.push(c),
                },
                '}' => {
                    if c != '}' {
                        return Err(static_err("Invalid syntax"));
                    } else {
                        state = '+';
                    }
                },
                sc @ _ => {
                    println!("Entered invalid state {}", sc);
                    panic!()
                },
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
