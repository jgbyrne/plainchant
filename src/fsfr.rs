use crate::fr;
use crate::util;
use bytes::{Bytes};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FSFileRack {
    file_dir: PathBuf,
    cache:    HashMap<String, Bytes>,
}

impl FSFileRack {
    pub fn from_dir(dir: &Path) -> Result<FSFileRack, util::PlainchantErr> {
        let fr_path = dir.join("rack").to_path_buf();
        match fr_path.is_dir() {
            true => Ok(FSFileRack { file_dir: fr_path,
                                    cache:    HashMap::new(), }),
            false => Err(fr::static_err("FS File Rack directory is not a directory")),
        }
    }

    fn thumb_id(file_id: &str) -> String {
        format!("{}_thumb.jpeg", file_id)
    }

    fn retrieve_file(&mut self, file_id: &str) -> Result<Bytes, util::PlainchantErr> {
        if let Some(bytes) = self.cache.get(file_id) {
            return Ok((*bytes).clone());
        }

        let f_res = File::open(self.file_dir.join(file_id));
        match f_res {
            Ok(f) => {
                let bytes_res = f.bytes().collect::<Result<Vec<u8>, std::io::Error>>();
                match bytes_res {
                    Ok(bytes) => {
                        let bytes = Bytes::from(bytes);
                        self.cache.insert(file_id.to_string(), bytes.clone());
                        Ok(bytes)
                    },
                    Err(_read_err) => {
                        Err(fr::static_err("Could not read bytes from requested file"))
                    },
                }
            },
            Err(_read_err) => Err(fr::static_err("Could not open requested file")),
        }
    }
}

impl fr::FileRack for FSFileRack {
    fn store_file(&mut self, file_id: &str, file: Bytes) -> Result<(), util::PlainchantErr> {
        let img = match image::load_from_memory(file.as_ref()) {
            Ok(img) => img,
            Err(_img_err) => return Err(fr::static_err("Could not handle file")),
        };

        let thumb = img.thumbnail(300, 300);

        let f_res = File::create(self.file_dir.join(file_id));
        match f_res {
            Ok(mut f) => {
                if f.write(&file).is_err() {
                    return Err(fr::static_err("Could not write to requested file"));
                }

                self.cache.insert(file_id.to_string(), file);
            },
            Err(_write_err) => return Err(fr::static_err("Could not open requested write file")),
        }

        let thumb_id = FSFileRack::thumb_id(file_id);
        let thumb_path = self.file_dir.join(&thumb_id);
        let mut thumb_buf: Vec<u8> = vec![];
        if thumb.write_to(&mut thumb_buf, image::ImageFormat::Jpeg)
                .is_err()
        {
            return Err(fr::static_err("Could not write thumbnail buffer"));
        }

        let thumbf_res = File::create(thumb_path);
        match thumbf_res {
            Ok(mut f) => {
                let thumb_buf = Bytes::from(thumb_buf);
                if f.write(&thumb_buf).is_err() {
                    Err(fr::static_err("Could not write to thumbnail file"))
                } else {
                    self.cache.insert(thumb_id, thumb_buf);
                    Ok(())
                }
            },
            Err(_write_err) => Err(fr::static_err("Could not open thumbnail file for writing")),
        }
    }

    fn get_file(&mut self, file_id: &str) -> Result<Bytes, util::PlainchantErr> {
        self.retrieve_file(file_id)
    }

    fn get_file_thumbnail(&mut self, file_id: &str) -> Result<Bytes, util::PlainchantErr> {
        self.retrieve_file(&FSFileRack::thumb_id(file_id))
    }

    fn delete_file(&mut self, file_id: &str) -> Result<(), util::PlainchantErr> {
        self.cache.remove(file_id);
        let thumb_id = FSFileRack::thumb_id(file_id);
        self.cache.remove(&thumb_id);

        let file_path = self.file_dir.join(file_id);
        if fs::remove_file(file_path).is_err() {
            return Err(fr::static_err("Could not delete file"));
        }
        let thumb_path = self.file_dir.join(&thumb_id);
        if fs::remove_file(thumb_path).is_err() {
            return Err(fr::static_err("Could not delete thumbnail file"));
        }

        Ok(())
    }
}
