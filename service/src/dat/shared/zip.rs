use log::debug;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tokio::task;
use zip::ZipArchive;

pub async fn extract_if_archived(path: &PathBuf) -> anyhow::Result<()> {
	if let Some(file_extension) = path.extension() {
		let file_extension = file_extension.to_str().unwrap_or_default();
		if file_extension == "zip" {
			debug!("Found zip file, extracting...");
			extract_zip_in_same_path(path).await?;
			debug!("Removing zip file: {path:?}");
			tokio::fs::remove_file(path).await?;
			debug!("Removed zip file");
		}
	}

	Ok(())
}

async fn extract_zip_in_same_path(path: &PathBuf) -> anyhow::Result<()> {
	let out = path.parent().unwrap().join(path.file_stem().unwrap());
	debug!("Extracting DAT(s) to: {:?}", &out);
	let path_owned = path.to_owned();
	task::spawn_blocking(move || extract_zip_to_directory(&path_owned, &out)).await??;

	Ok(())
}

fn extract_zip_to_directory(zip_path: &Path, out_dir: &Path) -> anyhow::Result<()> {
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
			if let Some(p) = out_path.parent()
				&& !p.exists()
			{
				fs::create_dir_all(p)?;
			}
			let mut outfile = File::create(&out_path)?;
			io::copy(&mut file, &mut outfile)?;
		}
	}

	Ok(())
}
