use snippext::{run, SnippetSettings, SnippetSource, error::SnippextError};
use structopt::StructOpt;
use config::{ConfigError, Config, File, Environment, Source, Value, FileFormat};
use std::env;
use std::collections::HashMap;
use std::path::PathBuf;


// static DEFAULT_CONFIG: &'static str = include_str!("default_snippext.yaml");


fn main() {
    let opt: Opt = Opt::from_args();

    // TODO: add debug
    // if opt.debug {
    //     std::env::set_var("RUST_LOG", "debug");
    //     env_logger::init();
    // }

    // https://stackoverflow.com/questions/27244465/merge-two-hashmaps-in-rust

    // Precedence of options
    // If you specify an option by using one of the environment variables described in this topic,
    // it overrides any value loaded from a profile in the configuration file.
    // If you specify an option by using a parameter on the AWS CLI command line, it overrides any
    // value from either the corresponding environment variable or a profile in the configuration file.


    // TODO: create settings by merging
    // default values / default config
    // project config / custom config from command line arg
    // environment vars
    // command line args


    let settings = build_settings(opt);

    // TODO: should be in lib?
    let validation_failures = validate_settings(&settings);




    // let mut s = Config::default();
    // // Start off by merging in the "default" configurations
    // // s.merge(File::with_name("config/default"))?;
    // // TODO: add defaults
    //
    // // s.merge(File::from_str(DEFAULT_CONFIG, FileFormat::Yaml)).unwrap();
    //
    // if let Some(config) = opt.config {
    //     s.merge(File::from(config)).unwrap();
    // } else {
    //     // TODO: use constant
    //     s.merge(File::with_name("snippext").required(false)).unwrap();
    // }
    //
    //
    // // TODO: this can probably come from structopt?
    // s.merge(Environment::with_prefix("snippext")).unwrap();
    //
    //
    // // TODO: add any command line args
    // // TODO: test that this works
    // // s.merge(opt).unwrap();
    //
    // if let Some(begin) = opt.begin {
    //     s.set("begin", begin);
    // }
    //
    // if let Some(end) = opt.end {
    //     s.set("end", end);
    // }
    //
    // if let Some(extension) = opt.extension {
    //     s.set("extension", extension);
    // }
    //
    // if let Some(comment_prefixes) = opt.comment_prefixes {
    //     s.set("comment_prefixes", comment_prefixes.0);
    // }
    //
    // if let Some(template) = opt.template {
    //     s.set("template", template);
    // }
    //
    // let snippet_source= if let Some(repo_url) = &opt.repository_url {
    //     SnippetSource::new_remote(
    //         repo_url.to_string(),
    //         opt.repository_branch.unwrap(),
    //         opt.repository_commit.clone(),
    //         opt.repository_directory.clone(),
    //         opt.sources.unwrap_or(Vec::new())
    //     )
    // } else {
    //     SnippetSource::new_local(opt.sources.unwrap_or(Vec::new()))
    // };
    //
    // // let sources: Vec<SnippetSource> = s.get("sources").unwrap();
    //
    // // TODO: might just have to append
    // // let sources_len = s.get_array("sources").unwrap().len();
    // // s.set(format!("sources[{}]", sources_len - 1).as_str(), snippet_source);
    //
    // if let Some(output_dir) = opt.output_dir {
    //     s.set("output_dir", output_dir.to_string());
    // }
    //
    // if let Some(targets) = opt.targets {
    //     s.set("targets", targets.clone());
    // }
    //
    //
    // // m.insert(String::from("begin"), Value::new(Some(&uri), self.begin.to_string()));
    // // m.insert(String::from("end"), Value::new(Some(&uri), self.end.to_string()));
    // // m.insert(String::from("extension"), Value::new(Some(&uri), self.extension.to_string()));
    // // m.insert(String::from("comment_prefixes"), Value::new(Some(&uri), self.comment_prefixes.0.clone()));
    // // m.insert(String::from("template"), Value::new(Some(&uri), self.template.to_string()));
    //
    // // let snippet_source= if let Some(repo_url) = &self.repository_url {
    // //     SnippetSource::new_remote(
    // //         repo_url.to_string(),
    // //         self.repository_branch.to_string(),
    // //         self.repository_commit.clone(),
    // //         self.repository_directory.clone(),
    // //         self.sources.clone()
    // //     )
    // // } else {
    // //     SnippetSource::new_local(self.sources.clone())
    // // };
    // //
    // // // m.insert(String::from("sources"), Value::new(Some(&uri), snippet_source));
    // //
    // // let json_string = serde_json::to_string(&snippet_source).unwrap();
    // // let map: HashMap<String, serde_json::Value> = serde_json::from_str(json_string.as_str()).unwrap();
    // // // let dict: Dictionary = serde_json::from_str(json_string).unwrap();
    // // m.insert(String::from("sources"), Value::new(Some(&uri), map));
    // //
    // //
    // // if let Some(output_dir) = &self.output_dir {
    // //     m.insert(String::from("output_dir"), Value::new(Some(&uri), output_dir.to_string()));
    // // }
    // //
    // // if let Some(targets) = &self.targets {
    // //     m.insert(String::from("targets"), Value::new(Some(&uri), targets.clone()));
    // // }
    //
    //
    //
    //
    //
    //
    //
    // let settings: SnippetSettings = s.try_into().unwrap();


    run(settings);

    // run(SnippetSettings::new(
    //     opt.comment_prefixes.0,
    //     opt.begin.to_owned(),
    //     opt.end.to_owned(),
    //     opt.output_dir,
    //     opt.extension.to_owned(),
    //     opt.template,
    //     opt.sources)
    // )
}

fn build_settings(opt: Opt) -> SnippetSettings {
    let mut s = Config::default();
    // Start off by merging in the "default" configurations
    // s.merge(File::with_name("config/default"))?;
    // TODO: add defaults

    if let Some(config) = opt.config {
        s.merge(File::from(config)).unwrap();
    } else {
        // TODO: use constant
        s.merge(File::with_name("snippext").required(false)).unwrap();
    }

    // TODO: this can probably come from structopt?
    s.merge(Environment::with_prefix("snippext")).unwrap();


    // TODO: add any command line args
    // TODO: test that this works
    // s.merge(opt).unwrap();

    if let Some(begin) = opt.begin {
        s.set("begin", begin);
    }

    if let Some(end) = opt.end {
        s.set("end", end);
    }

    if let Some(extension) = opt.extension {
        s.set("extension", extension);
    }

    if let Some(comment_prefixes) = opt.comment_prefixes {
        s.set("comment_prefixes", comment_prefixes);
    }

    if let Some(template) = opt.template {
        s.set("template", template);
    }

    if let Some(output_dir) = opt.output_dir {
        s.set("output_dir", output_dir);
    }

    if let Some(targets) = opt.targets {
        s.set("targets", targets);
    }



    // let snippet_source= if let Some(repo_url) = opt.repository_url {
    //     SnippetSource::new_remote(
    //         repo_url.to_string(),
    //         opt.repository_branch.unwrap(),
    //         opt.repository_commit.clone(),
    //         opt.repository_directory.clone(),
    //         opt.sources.unwrap_or(Vec::new())
    //     )
    // } else {
    //     SnippetSource::new_local(opt.sources.unwrap_or(Vec::new()))
    // };

    // let sources: Vec<SnippetSource> = s.get("sources").unwrap();
    // let json_string = serde_json::to_string(&snippet_source);
    // let map: HashMap<String, serde_json::Value> = serde_json::from_str(json_string.unwrap().as_str()).unwrap();
    // s.set(format!("sources[{}]", sources.len() - 1).as_str(), map);
    //

    // TODO: might just have to append
    // let sources_len = s.get_array("sources").unwrap().len();
    // s.set(format!("sources[{}]", sources_len - 1).as_str(), snippet_source);

    // if let Some(output_dir) = opt.output_dir {
    //     s.set("output_dir", output_dir.to_string());
    // }
    //
    // if let Some(targets) = opt.targets {
    //     s.set("targets", targets.clone());
    // }

    let mut settings: SnippetSettings = s.try_into().unwrap();
    let snippet_source= if let Some(repo_url) = opt.repository_url {
        SnippetSource::new_remote(
            repo_url.to_string(),
            opt.repository_branch.unwrap(),
            opt.repository_commit.clone(),
            opt.repository_directory.clone(),
            opt.sources.unwrap_or(Vec::new())
        )
    } else {
        SnippetSource::new_local(opt.sources.unwrap_or(Vec::new()))
    };

    settings.sources.push(snippet_source);

    return settings;

}

fn merge_configuration() -> Option<SnippetSettings> {
    // first lets look at configs

    // then look at env vars
    for (k, v) in std::env::vars() {
        // Treat empty environment variables as unset
        // if self.ignore_empty && value.is_empty() {
        //     continue;
        // }

        let mut key = k.to_lowercase();
        if k.starts_with("SNIPPEXT_") {
            // k.trim_start_matches("SNIPPEXT_").to_owned()
            // Remove this prefix from the key
            // key = key[prefix_pattern.len()..].to_string();
        }
    }

    // then use cli args
    None
}

/// returns a list of validation failures
fn validate_settings(settings: &SnippetSettings) -> Result<(), SnippextError> {
    let mut failures = Vec::new();

    if settings.comment_prefixes.is_empty() {
        failures.push(String::from("comment_prefixes must not be empty"));
    }

    if settings.sources.is_empty() {
        failures.push(String::from("sources must not be empty"));
    } else {
        for (i, source) in settings.sources.iter().enumerate() {
            if source.files.is_empty() {
                failures.push(format!("sources[{}].files must not be empty", i));
            }
        }
    }

    // TODO: should we output to stdout instead?
    if settings.output_dir.is_none() && settings.targets.is_none() {
        failures.push(String::from("output_dir or targets is required"));
    }

    return if failures.is_empty() {
        Err(SnippextError::ValidationError(failures))
    } else {
        Ok(())
    }
}

// https://github.com/viperproject/prusti-dev/blob/22a4eb83ef91391d9a91e6b3246ddf951b8eb251/prusti-common/src/config/commandline.rs#L97
impl Source for Opt {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new((*self).clone())
    }

    fn collect(&self) -> Result<HashMap<String, Value>, ConfigError> {
        let mut m = HashMap::new();
        let uri: String = "command line".into();

        // m.insert(String::from("begin"), Value::new(Some(&uri), self.begin.to_string()));
        // m.insert(String::from("end"), Value::new(Some(&uri), self.end.to_string()));
        // m.insert(String::from("extension"), Value::new(Some(&uri), self.extension.to_string()));
        // m.insert(String::from("comment_prefixes"), Value::new(Some(&uri), self.comment_prefixes.0.clone()));
        // m.insert(String::from("template"), Value::new(Some(&uri), self.template.to_string()));
        //
        // let snippet_source= if let Some(repo_url) = &self.repository_url {
        //     SnippetSource::new_remote(
        //         repo_url.to_string(),
        //         self.repository_branch.to_string(),
        //         self.repository_commit.clone(),
        //         self.repository_directory.clone(),
        //         self.sources.unwrap_or(Vec::new())
        //     )
        // } else {
        //     SnippetSource::new_local(self.sources.unwrap_or(Vec::new()))
        // };

        // m.insert(String::from("sources"), Value::new(Some(&uri), snippet_source));

        // let json_string = serde_json::to_string(&snippet_source).unwrap();
        // let map: HashMap<String, serde_json::Value> = serde_json::from_str(json_string.as_str()).unwrap();
        // let dict: Dictionary = serde_json::from_str(json_string).unwrap();
        // m.insert(String::from("sources"), Value::new(Some(&uri), map));


        if let Some(output_dir) = &self.output_dir {
            m.insert(String::from("output_dir"), Value::new(Some(&uri), output_dir.to_string()));
        }

        if let Some(targets) = &self.targets {
            m.insert(String::from("targets"), Value::new(Some(&uri), targets.clone()));
        }

        // TODO: I dont think we can have automatic defaults on structopt as we wont be able to
        // properly determine if they were provided and if they should values should be overwritten

        Ok(m)
    }
}

// split into subcommands??
// 1. generate - output to dir
// 2. write - write to target files
// 3. clean - clean up generate or files

// use constants that can also be used as defaults
// https://github.com/TeXitoi/structopt/issues/226

// TODO: given we arent defaulting we can delete this
// Only way I know how to get structopt default value to work with Vec is to use a struct
// #[derive(Clone, Debug, PartialEq)]
// struct CommentPrefixes(Vec<String>);
//
// impl std::str::FromStr for CommentPrefixes {
//     type Err = Box<dyn std::error::Error>;
//
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         Ok(CommentPrefixes(s.split(",").map(|x| x.trim().to_owned()).collect()))
//     }
// }

// TODO: environment variable fallback
#[derive(Clone, StructOpt, Debug)]
#[structopt(about = "TODO: add some details")]
struct Opt {

    #[structopt(
        short,
        long,
        parse(from_os_str),
        help = "Config file to use"
    )]
    config: Option<PathBuf>,

    #[structopt(
        short,
        long,
        help = "flag to mark beginning of a snippet"
    )]
    begin: Option<String>,

    #[structopt(
        short = "end",
        long,
        help = "flag to mark ending of a snippet"
    )]
    end: Option<String>,

    #[structopt(
        short = "x",
        long,
        help = "extension for generated files"
    )]
    extension: Option<String>,

    // TODO: make vec default to ["// ", "<!-- "]
    // The tag::[] and end::[] directives should be placed after a line comment as defined by the language of the source file.
    // comment prefix
    #[structopt(
        short = "p",
        long,
        help = "Prefixes to use for comments"
    )]
    comment_prefixes: Option<Vec<String>>,

    #[structopt(
        short,
        long,
        help = ""
    )]
    template: Option<String>,

    #[structopt(
        short,
        long,
        help = ""
    )]
    repository_url: Option<String>,

    #[structopt(
        short = "B",
        long,
        requires = "repository_url",
        help = ""
    )]
    repository_branch: Option<String>,

    #[structopt(
        short = "C",
        long,
        help = ""
    )]
    repository_commit: Option<String>,

    #[structopt(
        short = "D",
        long,
        help = "Directory remote repository is cloned into"
    )]
    repository_directory: Option<String>,

    // TODO: require if for output_dir an targets. one must be provided.

    #[structopt(
        short,
        long,
        required_unless = "targets",
        help = "directory in which the files will be generated"
    )]
    output_dir: Option<String>,

    // globs
    #[structopt(
        short = "T",
        long,
        required_unless = "output_dir",
        help = "The local directories that contain the files to be spliced with the code snippets."
    )]
    targets: Option<Vec<String>>,

    // TODO: write to target files instead of output directory

    // aka files
    // list of globs and default to all??
    // default to **
    #[structopt(
        short,
        long,
        help = "TODO: ..."
    )]
    sources: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::{Opt};

    #[test]
    fn default_config_file() {
        let opt = Opt {
            config: None,
            begin: None,
            end: None,
            extension: None,
            comment_prefixes: None,
            template: None,
            repository_url: None,
            repository_branch: None,
            repository_commit: None,
            repository_directory: None,
            output_dir: None,
            targets: None,
            sources: Some(vec![])
        };

        let settings = super::build_settings(opt);
        println!("{:?}", settings);
    }

    #[test]
    fn verify_cli_args() {
        let opt = Opt {
            config: None,
            begin: Some(String::from("snippext::")),
            end: Some(String::from("finish::")),
            extension: Some(String::from("txt")),
            comment_prefixes: Some(vec![String::from("# ")]),
            template: Some(String::from("````\n{{snippet}}\n```")),
            repository_url: Some(String::from("https://github.com/doctavious/snippext.git")),
            repository_branch: Some(String::from("main")),
            repository_commit: Some(String::from("1883d49216b34baed67629c363b40da3ead770b8")),
            repository_directory: Some(String::from("docs")),
            sources: Some(vec![String::from("**/*.rs")]),
            output_dir: Some(String::from("./snppext/")),
            targets: Some(vec![String::from("README.md")]),
        };

        let settings = super::build_settings(opt);

        assert_eq!("snippext::", settings.begin);
        assert_eq!("finish::", settings.end);
        assert_eq!("txt", settings.extension);
        assert_eq!(vec![String::from("# ")], settings.comment_prefixes);
        assert_eq!("````\n{{snippet}}\n```", settings.template);
        assert_eq!(Some(String::from("./snppext/")), settings.output_dir);
        assert_eq!(Some(vec![String::from("README.md")]), settings.targets);

        assert_eq!(2, settings.sources.len());
        let source = settings.sources.get(1).unwrap();
        assert_eq!(Some(String::from("https://github.com/doctavious/snippext.git")), source.repository);
        assert_eq!(Some(String::from("main")), source.branch);
        assert_eq!(Some(String::from("1883d49216b34baed67629c363b40da3ead770b8")), source.starting_point);
        assert_eq!(Some(String::from("docs")), source.directory);
        assert_eq!(vec![String::from("**/*.rs")], source.files);
    }

    #[test]
    fn support_overrides() {
        dotenv::from_path("./tests/.env.test").unwrap();

        let opt = Opt {
            config: Some(PathBuf::from("./tests/custom_snippext.yaml")),
            begin: None,
            end: None,
            extension: Some(String::from("txt")),
            comment_prefixes: None,
            template: None,
            repository_url: None,
            repository_branch: None,
            repository_commit: None,
            repository_directory: None,
            output_dir: None,
            targets: None,
            sources: None
        };

        let settings = super::build_settings(opt);
        // env overrides config
        assert_eq!(Some(String::from("./generated-snippets/")), settings.output_dir);
        // cli arg overrides env
        assert_eq!("txt", settings.extension);
    }

    // TODO: test valiations
}
