use std::{borrow::Cow, fs::File, io::{BufRead, BufReader, Cursor, Error, ErrorKind, Read, Seek}, path::{Path, PathBuf}, sync::Arc};
use rc_zip_sync::{EntryReader, ReadZip, ArchiveHandle};

pub trait AssetSource {
    type Reader: BufRead + Seek;
    fn get_reader(&self, path: &Path) -> std::io::Result<Self::Reader>;
    fn get_bytes(&self, path: &Path) -> std::io::Result<Box<[u8]>>;
}

#[derive(Clone, Debug)]
pub struct LocalAssetFolder {
    pub base_path: PathBuf,
}

impl LocalAssetFolder {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let base_path: &Path = path.as_ref();
        LocalAssetFolder {
            base_path: PathBuf::from(&base_path),
        }
    }
}

impl AssetSource for LocalAssetFolder {
    type Reader = BufReader<File>;
    fn get_reader(&self, path: &Path) -> std::io::Result<Self::Reader> {
        let path: PathBuf = [&self.base_path, path].iter().collect();
        let file = File::open(&path)?;
        Ok(BufReader::new(file))
    }

    fn get_bytes(&self, path: &Path) -> std::io::Result<Box<[u8]>> {
        let path: PathBuf = [&self.base_path, path].iter().collect();
        let contents = std::fs::read(path)?;
        Ok(contents.into_boxed_slice())
    }
}

pub struct LoadedZipBundle<'a> {
    pub archive: ArchiveHandle<'a, &'a[u8]>,
}

impl<'a> LoadedZipBundle<'a> {
    pub fn new(data: &'a &'a[u8]) -> std::io::Result<Self> {
        let archive = data.read_zip()?;
        Ok(LoadedZipBundle{archive})
    }
}

impl AssetSource for LoadedZipBundle<'_>{
    type Reader = Cursor<Box<[u8]>>;

    fn get_reader(&self, path: &Path) -> std::io::Result<Self::Reader> {
        let bytes = self.get_bytes(path)?;
        Ok(Cursor::new(bytes))
    }

    fn get_bytes(&self, path: &Path) -> std::io::Result<Box<[u8]>> {
        let file= self.archive.by_name(path.to_string_lossy()).ok_or(Error::from(ErrorKind::NotFound))?;
        let bytes = file.bytes()?;
        Ok(bytes.into_boxed_slice())
    }
}