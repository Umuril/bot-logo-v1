use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, Writer};
use std::io::Cursor;

const DISALLOWED_ELEMENTS: &[&str] = &["script", "foreignObject", "iframe"];

fn is_disallowed_attr(name: &str, value: &str) -> bool {
    let lower_name = name.to_ascii_lowercase();
    if lower_name.starts_with("on") {
        return true;
    }
    if lower_name == "href" || lower_name == "xlink:href" {
        let trimmed = value.trim();
        return !(trimmed.starts_with('#') || trimmed.starts_with("data:"));
    }
    false
}

fn filter_attrs<'a>(e: &BytesStart<'a>) -> BytesStart<'a> {
    let mut new_elem = BytesStart::new(String::from_utf8_lossy(e.name().as_ref()).to_string());
    for attr in e.attributes().flatten() {
        let name = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let value = attr.unescape_value().unwrap_or_default().to_string();
        if !is_disallowed_attr(&name, &value) {
            new_elem.push_attribute((name.as_str(), value.as_str()));
        }
    }
    new_elem
}

pub fn sanitize(input: &str) -> String {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut skip_depth: u32 = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if DISALLOWED_ELEMENTS.iter().any(|d| d.eq_ignore_ascii_case(&name)) {
                    skip_depth += 1;
                    continue;
                }
                if skip_depth > 0 {
                    continue;
                }
                writer.write_event(Event::Start(filter_attrs(&e))).unwrap();
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if DISALLOWED_ELEMENTS.iter().any(|d| d.eq_ignore_ascii_case(&name)) {
                    continue;
                }
                if skip_depth > 0 {
                    continue;
                }
                writer.write_event(Event::Empty(filter_attrs(&e))).unwrap();
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if DISALLOWED_ELEMENTS.iter().any(|d| d.eq_ignore_ascii_case(&name)) {
                    skip_depth = skip_depth.saturating_sub(1);
                    continue;
                }
                if skip_depth > 0 {
                    continue;
                }
                writer.write_event(Event::End(e)).unwrap();
            }
            Ok(event) => {
                if skip_depth == 0 {
                    writer.write_event(event).unwrap();
                }
            }
            Err(_) => break,
        }
    }

    String::from_utf8(writer.into_inner().into_inner()).unwrap_or_default()
}

pub fn render_png(sanitized_svg: &str) -> anyhow::Result<Vec<u8>> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(sanitized_svg, &options)?;
    let size = tree.size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32)
        .ok_or_else(|| anyhow::anyhow!("invalid SVG dimensions"))?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(Into::into)
}
