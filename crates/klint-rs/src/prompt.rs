use inquire::{InquireError, MultiSelect, Select};
use std::io::IsTerminal;

const AGENTS_QUESTION: &str = "Which agents should the skill be installed for?";
const STORAGE_QUESTION: &str = "How should the skill be stored?";
const SHARED_LABEL: &str =
    "Shared (recommended) — one skill in .agents/skills, other agents symlink to it";
const COPY_LABEL: &str = "Separate copies — an independent copy in every agent directory";

pub struct InstallChoices {
    pub agents: Vec<String>,
    pub shared: bool,
}

pub fn stdin_is_a_terminal() -> bool {
    std::io::stdin().is_terminal()
}

pub fn ask_install_choices(agents: &[&str]) -> Result<Option<InstallChoices>, String> {
    let Some(selected) = answered(
        MultiSelect::new(AGENTS_QUESTION, agents.to_vec())
            .with_all_selected_by_default()
            .prompt(),
    )?
    else {
        return Ok(None);
    };

    if selected.is_empty() {
        return Ok(Some(InstallChoices {
            agents: Vec::new(),
            shared: true,
        }));
    }

    let Some(storage) =
        answered(Select::new(STORAGE_QUESTION, vec![SHARED_LABEL, COPY_LABEL]).prompt())?
    else {
        return Ok(None);
    };

    Ok(Some(InstallChoices {
        agents: selected.into_iter().map(str::to_string).collect(),
        shared: storage == SHARED_LABEL,
    }))
}

fn answered<T>(result: Result<T, InquireError>) -> Result<Option<T>, String> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(None),
        Err(err) => Err(format!("klint: {err}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shared_option_is_offered_first_and_marked_recommended() {
        let options = [SHARED_LABEL, COPY_LABEL];
        assert!(options[0].starts_with("Shared"));
        assert!(options[0].contains("recommended"));
        assert!(options[0].contains(".agents/skills"));
    }

    #[test]
    fn a_cancelled_prompt_is_not_an_error() {
        let cancelled: Result<u8, InquireError> = Err(InquireError::OperationCanceled);
        let interrupted: Result<u8, InquireError> = Err(InquireError::OperationInterrupted);

        assert_eq!(answered(cancelled), Ok(None));
        assert_eq!(answered(interrupted), Ok(None));
    }

    #[test]
    fn a_terminal_failure_is_reported() {
        let failed: Result<u8, InquireError> =
            Err(InquireError::InvalidConfiguration("bad".to_string()));

        assert!(answered(failed).is_err());
    }

    #[test]
    fn piped_stdin_is_not_a_terminal() {
        assert!(!stdin_is_a_terminal());
    }
}
