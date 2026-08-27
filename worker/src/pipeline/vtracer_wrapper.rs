use std::path::Path;

pub fn png_to_svg(png_path: &Path, svg_path: &Path) -> anyhow::Result<()> {
    let config = vtracer::Config::default();
    vtracer::convert_image_to_svg(png_path, svg_path, config).map_err(|err| anyhow::anyhow!("vtracer conversion failed: {err}"))
}
