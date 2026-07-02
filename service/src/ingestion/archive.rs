use anyhow::bail;
use log::debug;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tokio::task;
use zip::ZipArchive;

const MAX_ZIP_ENTRY_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ZIP_TOTAL_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_ZIP_ENTRIES: usize = 10_000;

pub async fn extract_if_archived(path: &PathBuf) -> anyhow::Result<()> {
	if let Some(file_extension) = path.extension() {
		let file_extension = file_extension.to_str().unwrap_or_default();
		if file_extension == "zip" {
			debug!("Found zip file, extracting");
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

	if archive.len() > MAX_ZIP_ENTRIES {
		bail!(
			"zip archive contains {} entries which exceeds the {MAX_ZIP_ENTRIES} limit",
			archive.len()
		);
	}

	let mut total_written: u64 = 0;

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
			// Read one byte past the cap so hitting the cap is detectable after the copy.
			let limit = MAX_ZIP_ENTRY_BYTES + 1;
			let mut limited = Read::take(&mut file, limit);
			let written = io::copy(&mut limited, &mut outfile)?;
			if written > MAX_ZIP_ENTRY_BYTES {
				bail!("zip entry exceeds {MAX_ZIP_ENTRY_BYTES} bytes decompressed; aborting");
			}
			total_written = total_written.saturating_add(written);
			if total_written > MAX_ZIP_TOTAL_BYTES {
				bail!("zip archive exceeds {MAX_ZIP_TOTAL_BYTES} bytes decompressed; aborting");
			}
		}
	}

	Ok(())
}
