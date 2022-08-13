use crate::fr;
use crate::util;
use bytes::Bytes;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[derive(Debug)]
pub struct FSFileRack {
    file_dir: PathBuf,
    cache:    RwLock<HashMap<String, Bytes>>,
}

impl FSFileRack {
    pub fn from_dir(dir: &Path) -> Result<FSFileRack, util::PlainchantErr> {
        if !dir.is_dir() {
            return Err(fr::static_err("FS File Rack directory is not a directory"));
        }

        let fr_path = dir.join("rack").to_path_buf();

        if !fr_path.is_dir() {
            if !fs::create_dir(&fr_path).is_ok() {
                return Err(fr::static_err("Failed to create fsfr /rack directory"));
            }
        }

        Ok(FSFileRack {
            file_dir: fr_path,
            cache:    RwLock::new(HashMap::new()),
        })
    }

    fn thumb_id(file_id: &str) -> String {
        format!("{}_thumb.jpeg", file_id)
    }

    fn retrieve_file(&self, file_id: &str) -> Result<Bytes, util::PlainchantErr> {
        {
            let cg = self
                .cache
                .read()
                .map_err(|_| fr::static_err("Could not gain read access to file cache"))?;
            if let Some(bytes) = cg.get(file_id) {
                return Ok((*bytes).clone());
            }
        }

        let f_res = File::open(self.file_dir.join(file_id));
        match f_res {
            Ok(f) => {
                let bytes_res = f.bytes().collect::<Result<Vec<u8>, std::io::Error>>();
                match bytes_res {
                    Ok(bytes) => {
                        let bytes = Bytes::from(bytes);
                        {
                            let mut cg = self.cache.write().map_err(|_| {
                                fr::static_err("Could not gain write access to file cache")
                            })?;
                            let cache = cg.deref_mut();
                            cache.insert(file_id.to_string(), bytes.clone());
                        }
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
    fn store_file(&self, file_id: &str, file: Bytes) -> Result<(), util::PlainchantErr> {
        let img = image::load_from_memory(file.as_ref())
            .map_err(|_| fr::static_err("Could not handle file"))?;

        let thumb = img.thumbnail(300, 300);

        let mut fd = File::create(self.file_dir.join(file_id))
            .map_err(|_| fr::static_err("Could not open requested write file"))?;

        fd.write(&file)
            .map_err(|_| fr::static_err("Could not write to requested file"))?;

        let thumb_id = FSFileRack::thumb_id(file_id);
        let thumb_path = self.file_dir.join(&thumb_id);

        let mut thumb_buf: Vec<u8> = vec![];
        thumb
            .write_to(&mut Cursor::new(&mut thumb_buf), image::ImageFormat::Jpeg)
            .map_err(|_| fr::static_err("Could not write thumbnail buffer"))?;
        let thumb_buf = Bytes::from(thumb_buf);

        let mut thumbfd = File::create(thumb_path)
            .map_err(|_| fr::static_err("Could not open thumbnail file for writing"))?;
        thumbfd
            .write(&thumb_buf)
            .map_err(|_| fr::static_err("Could not write to thumbnail file"))?;

        {
            let mut cg = self
                .cache
                .write()
                .map_err(|_| fr::static_err("Could not gain write access to file cache"))?;
            let cache = cg.deref_mut();
            cache.insert(file_id.to_string(), file);
            cache.insert(thumb_id, thumb_buf);
        }

        Ok(())
    }

    fn get_file(&self, file_id: &str) -> Result<Bytes, util::PlainchantErr> {
        self.retrieve_file(file_id)
    }

    fn get_file_thumbnail(&self, file_id: &str) -> Result<Bytes, util::PlainchantErr> {
        self.retrieve_file(&FSFileRack::thumb_id(file_id))
    }

    fn delete_file(&self, file_id: &str) -> Result<(), util::PlainchantErr> {
        let thumb_id = FSFileRack::thumb_id(file_id);
        {
            let mut cg = self
                .cache
                .write()
                .map_err(|_| fr::static_err("Could not gain write access to file cache"))?;
            let cache = cg.deref_mut();
            cache.remove(file_id);
            cache.remove(&thumb_id);
        }

        let file_path = self.file_dir.join(file_id);
        fs::remove_file(file_path).map_err(|_| fr::static_err("Could not delete file"))?;
        let thumb_path = self.file_dir.join(&thumb_id);
        fs::remove_file(thumb_path)
            .map_err(|_| fr::static_err("Could not delete thumbnail file"))?;

        Ok(())
    }
}
