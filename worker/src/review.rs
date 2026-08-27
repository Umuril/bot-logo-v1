use std::path::Path;
use std::process::Command;

#[derive(Debug, PartialEq)]
pub enum Decision {
    Accept,
    Retry,
    RetryWithPrompt(String),
    Abandon,
}

pub fn prompt_for_decision(image_path: &Path) -> anyhow::Result<Decision> {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = Command::new(opener).arg(image_path).status();

    println!(
        "Reviewing {}: [a]ccept / [r]etry / [e]dit prompt & retry / [any other key] abandon",
        image_path.display()
    );
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(match input.trim().to_ascii_lowercase().as_str() {
        "a" | "accept" => Decision::Accept,
        "r" | "retry" => Decision::Retry,
        "e" | "edit" => {
            println!("Enter the new prompt:");
            let mut new_prompt = String::new();
            std::io::stdin().read_line(&mut new_prompt)?;
            Decision::RetryWithPrompt(new_prompt.trim().to_string())
        }
        _ => Decision::Abandon,
    })
}
