use super::GenerationRequest;

const SYSTEM_PROMPT: &str = "You are an expert vector logo designer. You respond with ONLY raw SVG markup for a single, self-contained logo mark \u{2014} no prose, no explanation, no markdown code fences. The SVG must be flat and geometric (solid shapes, simple paths, optional <text>), use viewBox=\"0 0 512 512\", contain exactly one logo (never a grid, collage, contact sheet, or multiple variations), and avoid <script>, <foreignObject>, <iframe>, event-handler attributes (onclick, etc.), and external href references.";

pub fn system_prompt() -> &'static str {
    SYSTEM_PROMPT
}

pub fn build_svg_prompt(request: &GenerationRequest) -> String {
    let mut prompt = format!("Design a logo for: {}", request.prompt);
    if let Some(reference_path) = &request.reference_svg_path {
        if let Ok(reference_svg) = std::fs::read_to_string(reference_path) {
            prompt.push_str(&format!(
                "\n\nHere is a previous candidate to use as a starting point \u{2014} produce a new variation of it:\n{reference_svg}"
            ));
        }
    }
    prompt
}

/// Strips a leading/trailing markdown code fence (e.g. ```svg ... ```), if present.
fn strip_markdown_fences(text: &str) -> String {
    let trimmed = text.trim();
    let Some(newline_idx) = trimmed.strip_prefix("```").and_then(|_| trimmed.find('\n')) else {
        return trimmed.to_string();
    };
    trimmed[newline_idx + 1..].strip_suffix("```").unwrap_or(&trimmed[newline_idx + 1..]).trim().to_string()
}

/// Cleans an LLM's raw text response into SVG markup, failing if it doesn't look like SVG at all.
pub fn extract_svg(raw: &str) -> anyhow::Result<String> {
    let cleaned = strip_markdown_fences(raw);
    anyhow::ensure!(cleaned.to_ascii_lowercase().contains("<svg"), "model output does not contain an <svg> element: {cleaned:?}");
    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_svg_prompt_without_reference() {
        let request = GenerationRequest { prompt: "a fox".to_string(), reference_svg_path: None, reference_png_path: None };
        let prompt = build_svg_prompt(&request);
        assert_eq!(prompt, "Design a logo for: a fox");
    }

    #[test]
    fn build_svg_prompt_with_reference() {
        let dir = std::env::temp_dir().join("worker-svg-prompt-test");
        std::fs::create_dir_all(&dir).unwrap();
        let reference_path = dir.join("reference.svg");
        std::fs::write(&reference_path, "<svg>ref</svg>").unwrap();

        let request =
            GenerationRequest { prompt: "a fox".to_string(), reference_svg_path: Some(reference_path), reference_png_path: None };
        let prompt = build_svg_prompt(&request);
        assert!(prompt.contains("Design a logo for: a fox"));
        assert!(prompt.contains("<svg>ref</svg>"));
    }

    #[test]
    fn extract_svg_passes_through_plain_svg() {
        let svg = extract_svg("<svg viewBox=\"0 0 512 512\"></svg>").unwrap();
        assert_eq!(svg, "<svg viewBox=\"0 0 512 512\"></svg>");
    }

    #[test]
    fn extract_svg_strips_fenced_code_block() {
        let raw = "```svg\n<svg><circle/></svg>\n```";
        let svg = extract_svg(raw).unwrap();
        assert_eq!(svg, "<svg><circle/></svg>");
    }

    #[test]
    fn extract_svg_strips_bare_fence_without_language_tag() {
        let raw = "```\n<svg><rect/></svg>\n```";
        let svg = extract_svg(raw).unwrap();
        assert_eq!(svg, "<svg><rect/></svg>");
    }

    #[test]
    fn extract_svg_rejects_non_svg_output() {
        assert!(extract_svg("Sorry, I can't help with that.").is_err());
    }
}
