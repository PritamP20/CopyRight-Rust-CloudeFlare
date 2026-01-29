use anyhow::{Context, Result};
use image::io::Reader as ImageReader;
use img_hash::{HasherConfig, ImageHash};
use std::path::Path;

pub fn compute_phash(image_path: &Path) -> Result<ImageHash> {
    let image = ImageReader::open(image_path)
        .context(format!("Failed to open image: {:?}", image_path))?
        .decode()
        .context("Failed to decode image")?;

    let hasher = HasherConfig::new().to_hasher();
    let hash = hasher.hash_image(&image);

    Ok(hash)
}
