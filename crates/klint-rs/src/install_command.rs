use crate::command::{Leaf, OptionSpec, Values};
use crate::prompt;
use klint_rs::{InstallRequest, install_skill, known_agent_names, known_agents};

pub(crate) fn install_skill_leaf() -> Leaf {
    Leaf {
        summary: "Install the embedded klint-rules skill into agent config directories",
        options: vec![
            OptionSpec::valued(
                "agents",
                "<list>",
                format!("Comma-separated: {} (default: all)", known_agent_names()),
            ),
            OptionSpec::flag(
                "symlink",
                "Keep one skill in .agents/skills and symlink the others to it (default)",
            )
            .aliases(&["--shared"]),
            OptionSpec::flag("copy", "Give every agent directory its own copy"),
            OptionSpec::flag("force", "Replace a skill directory klint did not install"),
        ],
        positionals: None,
        details: vec![
            "Asks which agents and how to store the skill when run on a terminal without"
                .to_string(),
            "--agents, --symlink or --copy.".to_string(),
        ],
    }
}

pub(crate) fn install_skill_command(values: &Values) -> i32 {
    let (mut request, decided) = match install_request(values) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("{err}");

            return 1;
        }
    };
    if should_prompt(decided, prompt::stdin_is_a_terminal()) {
        request = match chosen_request(request, prompt::ask_install_choices(&known_agents())) {
            Ok(chosen) => chosen,
            Err(code) => return code,
        };
    }

    match install_skill(&request) {
        Ok(installed) => {
            for line in installed {
                println!("klint: {line}");
            }

            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn should_prompt(decided: bool, terminal: bool) -> bool {
    !decided && terminal
}

fn chosen_request(
    mut request: InstallRequest,
    answer: Result<Option<prompt::InstallChoices>, String>,
) -> Result<InstallRequest, i32> {
    match answer {
        Ok(Some(choices)) if choices.agents.is_empty() => {
            println!("klint: no agents selected — nothing to install");

            Err(0)
        }
        Ok(Some(choices)) => {
            request.agents = choices.agents;
            request.shared = choices.shared;

            Ok(request)
        }
        Ok(None) => {
            println!("klint: cancelled");

            Err(0)
        }
        Err(err) => {
            eprintln!("{err}");

            Err(1)
        }
    }
}

fn install_request(values: &Values) -> Result<(InstallRequest, bool), String> {
    let root =
        std::env::current_dir().map_err(|err| format!("klint: failed to resolve cwd: {err}"))?;
    let agents: Vec<String> = values
        .text("agents")
        .unwrap_or_default()
        .split(',')
        .map(|agent| agent.trim().to_string())
        .filter(|agent| !agent.is_empty())
        .collect();
    let decided = values.text("agents").is_some() || values.flag("symlink") || values.flag("copy");
    Ok((
        InstallRequest {
            root,
            agents,
            shared: !values.flag("copy"),
            force: values.flag("force"),
        },
        decided,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{Dispatch, dispatch};
    use crate::klint_command;

    fn install_values(args: &[&str]) -> Values {
        let args: Vec<String> = std::iter::once("install-skill")
            .chain(args.iter().copied())
            .map(str::to_string)
            .collect();

        match dispatch(&klint_command(), &args, "klint") {
            Dispatch::Run { values, .. } => values,
            Dispatch::Exit(code) => panic!("install-skill {args:?} exited with {code}"),
        }
    }

    fn request_for(args: &[&str]) -> (InstallRequest, bool) {
        install_request(&install_values(args)).expect("request")
    }

    #[test]
    fn any_agents_or_storage_flag_decides_without_a_prompt() {
        assert!(!request_for(&[]).1);
        assert!(request_for(&["--agents", "claude"]).1);
        assert!(request_for(&["--symlink"]).1);
        assert!(request_for(&["--copy"]).1);
    }

    #[test]
    fn copy_turns_off_the_shared_skill() {
        assert!(request_for(&[]).0.shared);
        assert!(!request_for(&["--copy"]).0.shared);
    }

    #[test]
    fn prompts_only_an_undecided_terminal_user() {
        assert!(should_prompt(false, true));
        assert!(!should_prompt(true, true));
        assert!(!should_prompt(false, false));
    }

    fn choices(agents: &[&str], shared: bool) -> Result<Option<prompt::InstallChoices>, String> {
        Ok(Some(prompt::InstallChoices {
            agents: agents.iter().map(|agent| agent.to_string()).collect(),
            shared,
        }))
    }

    #[test]
    fn chosen_agents_and_storage_replace_the_request() {
        let chosen = chosen_request(request_for(&[]).0, choices(&["cursor"], false))
            .unwrap_or_else(|code| panic!("exited with {code}"));

        assert_eq!(chosen.agents, vec!["cursor"]);
        assert!(!chosen.shared);
    }

    #[test]
    fn no_agents_cancel_or_a_prompt_error_stop_the_install() {
        let request = || request_for(&[]).0;

        assert_eq!(chosen_request(request(), choices(&[], true)).err(), Some(0));
        assert_eq!(chosen_request(request(), Ok(None)).err(), Some(0));
        assert_eq!(
            chosen_request(request(), Err("klint: no terminal".to_string())).err(),
            Some(1)
        );
    }
}
