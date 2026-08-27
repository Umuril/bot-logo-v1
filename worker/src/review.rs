use std::path::Path;
use std::process::Command;

#[derive(Debug, PartialEq)]
pub enum Decision {
    Accept,
    Retry,
    Abandon,
}

pub fn prompt_for_decision(image_path: &Path) -> anyhow::Result<Decision> {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = Command::new(opener).arg(image_path).status();

    println!("Reviewing {}: [a]ccept / [r]etry / [any other key] abandon", image_path.display());
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(match input.trim().to_ascii_lowercase().as_str() {
        "a" | "accept" => Decision::Accept,
        "r" | "retry" => Decision::Retry,
        _ => Decision::Abandon,
    })
}
