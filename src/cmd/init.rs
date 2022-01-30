use std::collections::{HashMap, HashSet};
use clap::{ArgMatches, Parser, Subcommand};
use config::Config;
use inquire::{Confirm, required, Select, Text};
use snippext::{DEFAULT_BEGIN, DEFAULT_COMMENT_PREFIXES, DEFAULT_END, DEFAULT_FILE_EXTENSION, DEFAULT_SOURCE_FILES, DEFAULT_TEMPLATE, init, InitSettings, SnippetSource, SnippextResult, SnippextSettings, SnippextTemplate};

#[derive(Clone, Debug, Parser)]
#[clap()]
pub struct InitOpt {
    #[clap(long, help = "TODO: ...")]
    pub default: bool,
}



pub fn execute(init_opt: InitOpt) -> SnippextResult<()> {
    let init_settings = if init_opt.default {
        None
    } else {
        Some(init_settings_from_prompt()?)
    };
    init(init_settings)
}

// &mut ui::backend::TestBackend::new((1, 1).into()),

// let mut backend = helpers::SnapshotOnFlushBackend::new((50, 20).into());
// let mut events = TestEvents::new(vec![
//     KeyCode::Char('t').into(),
//     KeyCode::Char('r').into(),
//     KeyCode::Enter.into(),
//     KeyCode::Home.into(),
//     KeyCode::Char('s').into(),
//     KeyCode::Enter.into(),
// ]);
//
// let ans = requestty::prompt_one_with(prompt, &mut backend, &mut events).unwrap();

// pub fn prompt_one<'a, I: Into<Question<'a>>>(question: I) -> Result<Answer> {
//     let stdout = std::io::stdout();
//     let mut stdout = ui::backend::get_backend(stdout.lock());
//     let mut events = ui::events::get_events();
//
//     prompt_one_with(question.into(), &mut stdout, &mut events)
// }
fn init_settings_from_prompt_internal() {

}

fn init_settings_from_prompt() -> SnippextResult<SnippextSettings> {
    // TODO: look at render config options

    let begin = Text::new("Begin tag:")
        .with_default(DEFAULT_BEGIN)
        .with_help_message("e.g. Music Store")
        .prompt()?;

    let end = Text::new("End tag:")
        .with_default(DEFAULT_END)
        .with_help_message("e.g. Music Store")
        .prompt()?;

    let extension = Text::new("Extension:")
        .with_default(DEFAULT_FILE_EXTENSION)
        .with_help_message("e.g. Music Store")
        .prompt()?;

    // TODO: custom type or loop?
    // TODO: validation
    let comment_prefixes: Vec<String> = Text::new("Comment prefixes:")
        .with_default(DEFAULT_COMMENT_PREFIXES.join(",").as_str())
        .with_help_message("Please include appropriate number of spaces if applicable")
        .prompt()?
        .split(",")
        .map(|s| s.to_string())
        .collect();


    // TODO: support multiple templates (id / default / template)
    let mut templates: HashMap<String, SnippextTemplate> = HashMap::new();
    loop {
        let identifier = Text::new("Template identifier:")
            .with_validator(required!("This field is required"))
            // .with_validator(&|id| {
            //     let now = chrono::Utc::now().naive_utc().date();
            //
            //     if d.ge(&now) {
            //         Ok(Validation::Invalid("Date must be in the past".into()))
            //     } else {
            //         Ok(Validation::Valid)
            //     }
            // })
            .with_help_message("e.g. Music Store")
            .prompt()?;

        let template = Text::new("Template content:")
            .with_default(DEFAULT_TEMPLATE)
            .with_help_message("e.g. Music Store")
            .prompt()?;


        // mark default? can we be smart of if already has a default then no need to ask.
        // if only one template then just mark that as default

        templates.insert(identifier, SnippextTemplate {
            content: template,
            default: false
        });

        let add_another_template = Confirm::new("Add another template?")
            .with_default(false)
            .with_help_message("")
            .prompt()?;

        if !add_another_template {
            break;
        }
    }

    // TODO: add if user wants to use default template?

    let mut sources: Vec<SnippetSource> = Vec::new();
    loop {

        let source_type = Select::new("Type of source?", vec!["local", "remote"])
            .prompt()?;

        if source_type.eq("local") {
            // TODO: loop or comma separated globs?
            let source_files = Text::new("Source files:")
                .with_default(DEFAULT_SOURCE_FILES)
                .with_help_message("Globs")
                .prompt()?;

            // sources.insert(SnippetSource::new_local())
        } else {
            // repository
            // branch
            // commit
            // directory
            // files

            // sources.insert(SnippetSource::new_remote())
        }

        let add_another_source = Confirm::new("Add another source?")
            .with_default(false)
            .with_help_message("")
            .prompt()?;

        if !add_another_source {
            break;
        }
    }

    let output_directory_prompt = Text::new("Output directory:")
        .with_help_message("e.g. Music Store")
        .prompt()?;
    let output_dir = if output_directory_prompt.is_empty() {
        Some(output_directory_prompt)
    } else {
        None
    };

    let targets = Text::new("targets:")
        .with_default(DEFAULT_COMMENT_PREFIXES.join(",").as_str())
        .with_help_message("e.g. Music Store")
        .prompt()?;

    Ok(SnippextSettings {
        begin,
        end,
        extension,
        comment_prefixes: HashSet::from_iter(comment_prefixes),
        templates: Default::default(),
        sources: vec![],
        output_dir,
        targets: None
    })
}

// TODO: do we need this?
fn build_init_settings(opt: InitOpt) -> SnippextResult<InitSettings> {
    let mut s = Config::default();
    let mut settings: InitSettings = s.try_into()?;
    return Ok(settings);
}
