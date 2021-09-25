use std::{fs, io};
use structopt::StructOpt;
use snippext::extract_snippets;
use std::path::{PathBuf, Path};
use walkdir::WalkDir;
use std::fs::File;

fn main() {
    let opt: Opt = Opt::from_args();

    // TODO: add debug
    // if opt.debug {
    //     std::env::set_var("RUST_LOG", "debug");
    //     env_logger::init();
    // }

    let filenames = get_filenames(opt.sources);
    for filename in filenames {
        let snippets = extract_snippets(
            opt.begin.to_owned(),
            opt.end.to_owned(),
            filename
        ).unwrap();

        for snippet in snippets {
            // TODO: include filename
            let output_path = Path::new(opt.output_dir.as_str())
                .join(snippet.identifier)
                .with_extension(opt.extension.as_str());

            // TODO: should we include a comment that the file is generated?
            fs::write(output_path, snippet.text).unwrap();
        }
    }

}

// if an entry is a directory all files from directory will be listed.
fn get_filenames(sources: Vec<String>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();

    for source in sources {
        let path = Path::new(&source);
        if !path.is_dir() {
            out.push(path.to_path_buf())
        }

        for entry in WalkDir::new(&source)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir()) {
            out.push(entry.path().to_path_buf());
        }
    }

    out
}

#[derive(StructOpt, Debug)]
#[structopt(about = "TODO: add some details")]
struct Opt {
    #[structopt(short, long, help = "flag to mark beginning of a snippet")]
    begin: String,

    #[structopt(short, long, help = "flag to mark ending of a snippet")]
    end: String,

    #[structopt(
        short,
        long,
        default_value = "./snippets/",
        help = "directory in which the files will be generated"
    )]
    output_dir: String,

    #[structopt(
        short = "ext",
        long,
        default_value = ".md",
        help = "extension for generated files"
    )]
    extension: String,

    // default to current directory
    sources: Vec<String>

    // excludes
    // includes
}
