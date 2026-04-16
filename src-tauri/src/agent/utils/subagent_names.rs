use std::collections::HashSet;

use rand::seq::SliceRandom;

use crate::utils::i18n::I18nManager;

pub fn subagent_name_candidates(existing_names: &HashSet<String>) -> Result<Vec<String>, String> {
    let mut library = I18nManager::get_string_array("subagent.names")
        .ok_or_else(|| "Missing i18n key: subagent.names".to_string())?;

    library.retain(|candidate| !existing_names.contains(candidate));
    if library.is_empty() {
        return Err("No available subagent names configured for current locale".to_string());
    }

    let mut rng = rand::thread_rng();
    library.shuffle(&mut rng);
    Ok(library)
}
