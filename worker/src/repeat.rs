use shared::ContextResponse;

pub fn find_prompt<'a>(context: &'a ContextResponse, short_name: &str) -> Option<&'a str> {
    context.candidates.iter().find(|c| c.short_name == short_name).map(|c| c.prompt.as_str())
}
