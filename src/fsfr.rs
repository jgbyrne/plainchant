use crate::fr;
use crate::util;
use bytes::Bytes;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Read, Write, Error};

#[derive(Debug)]
pub struct FSFileRack {
    file_dir: PathBuf
}

impl <'init> FSFileRack {
    pub fn from_dir(dir: &'init str) -> Result<FSFileRack, util::PlainchantErr> {
        let fr_path = Path::new(&dir).to_path_buf();
        match fr_path.is_dir() {
            true => Ok(FSFileRack { file_dir: fr_path }),
            false => Err(fr::static_err("FS File Rack directory is not a directory")),
        }
    }
}

impl fr::FileRack for FSFileRack {
    fn store_file(&self, file_id: &str, file: Bytes) -> Result<(), util::PlainchantErr> {
        let mut f_res = File::create(self.file_dir.join(file_id));
        match f_res {
            Ok(mut f) => {
                f.write(&file);
                Ok(())
            },
            Err(e) => Err(fr::static_err("Could not open requested write file")),
        }
    }

    fn get_file(&self, file_id: &str) -> Result<Bytes, util::PlainchantErr> {
        let mut f_res = File::open(self.file_dir.join(file_id));
        match f_res {
            Ok(f) => {
                let bytes_res = f.bytes().collect::<Result<Vec<u8>, std::io::Error>>();
                match bytes_res {
                    Ok(bytes) => Ok(Bytes::from(bytes)),
                    Err(e) => Err(fr::static_err("Could not read bytes from requested file")),
                }
            },
            Err(e) => Err(fr::static_err("Could not open requested file")),
        }
    }

    fn delete_file(&self, file_id: &str) -> Result<(), util::PlainchantErr> {
         unimplemented!();
    }
}
