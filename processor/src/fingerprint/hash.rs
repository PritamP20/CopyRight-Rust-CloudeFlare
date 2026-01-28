use anyhow::{Context, Result};
use image::io::Reader as ImageReader;
use img_hash::HasherConfig;
use std::path::Path;

pub fn compute_phash(image_path: &Path) -> Result<String> {
    let image = ImageReader::open(image_path)
        .context(format!("Failed to open image: {:?}", image_path))?
        .decode()
        .context("Failed to decode image")?;

    let hasher = HasherConfig::new().to_hasher();
    // The error suggests img_hash might be using a different version of image crate.
    // However, since we defined image = "0.24" and img_hash = "3.2" (which usually uses roughly that),
    // it could be a transient dependency issue.
    // But actually, img_hash 3.2 uses image 0.23.14.
    // We are using image 0.24. This is the mismatch.
    // We should either downgrade `image` to match `img_hash` or try to force compatibility.
    // Downgrading `image` in Cargo.toml is the safest bet to avoid two `image` versions.

    // For now, I will modify the code to assume we fix Cargo.toml in the next step.
    let hash = hasher.hash_image(&image);

    Ok(hash.to_base64())
}
