use shared::{CandidateInfo, ContextResponse};

pub fn find_candidate<'a>(context: &'a ContextResponse, short_name: &str) -> Option<&'a CandidateInfo> {
    context.candidates.iter().find(|c| c.short_name == short_name)
}
