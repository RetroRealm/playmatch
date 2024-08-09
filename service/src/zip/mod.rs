use std::fs::File;
use std::path::Path;
use std::{fs, io};

use zip::ZipArchive;

pub fn extract_zip_to_directory(zip_path: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => path,
            None => continue,
        };

        let out_path = out_dir.join(outpath);

        if file.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(p) = out_path.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = File::create(&out_path)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}
