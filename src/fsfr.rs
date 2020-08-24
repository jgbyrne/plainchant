use crate::fr;
use crate::util;
use bytes::Bytes;
use image;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FSFileRack {
    file_dir: PathBuf,
    cache:    HashMap<String, Bytes>,
}

impl<'init> FSFileRack {
    pub fn from_dir(dir: &'init str) -> Result<FSFileRack, util::PlainchantErr> {
        let fr_path = Path::new(&dir).to_path_buf();
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
        match self.cache.get(file_id) {
            Some(bytes) => return Ok((*bytes).clone()),
            None => {},
        }

        let f_res = File::open(self.file_dir.join(file_id));
        match f_res {
            Ok(f) => {
                let bytes_res = f.bytes().collect::<Result<Vec<u8>, std::io::Error>>();
                match bytes_res {
                    Ok(bytes) => Ok(Bytes::from(bytes)),
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
        match thumb.save_with_format(thumb_path, image::ImageFormat::Jpeg) {
            Ok(_) => {
                self.cache
                    .insert(thumb_id.to_string(), Bytes::from(thumb.to_bytes()));
                Ok(())
            },
            Err(_write_err) => return Err(fr::static_err("Could not write thumbnail file")),
        }
    }

    fn get_file(&mut self, file_id: &str) -> Result<Bytes, util::PlainchantErr> {
        self.retrieve_file(file_id)
    }

    fn get_file_thumbnail(&mut self, file_id: &str) -> Result<Bytes, util::PlainchantErr> {
        self.retrieve_file(&FSFileRack::thumb_id(file_id))
    }

    fn delete_file(&mut self, file_id: &str) -> Result<(), util::PlainchantErr> {
        unimplemented!();
    }
}
