// from https://github.com/kardeiz/sanitize-filename with some minor modifications

use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};

lazy_static! {
    static ref ILLEGAL_RE: Regex = Regex::new(r#"[/\?<>\\:\*\|":]"#).unwrap();
    static ref CONTROL_RE: Regex = Regex::new(r#"[\x00-\x1f\x80-\x9f]"#).unwrap();
    static ref RESERVED_RE: Regex = Regex::new(r#"^\.+$"#).unwrap();
    static ref WINDOWS_RESERVED_RE: Regex = RegexBuilder::new(r#"(?i)^(con|prn|aux|nul|com[0-9]|lpt[0-9])(\..*)?$"#)
        .case_insensitive(true)
        .build()
        .unwrap();
    static ref WINDOWS_TRAILING_RE: Regex = Regex::new(r#"^\.+$"#).unwrap();
}

#[derive(Clone)]
struct Options {
    windows: bool
}

impl Default for Options {
    fn default() -> Self {
        Options {
            windows: cfg!(windows),
        }
    }
}

pub(crate) fn sanitize<S: AsRef<str>>(name: S) -> String {
    sanitize_with_options(name, Options::default())
}

fn sanitize_with_options<S: AsRef<str>>(name: S, options: Options) -> String {

    let name = name.as_ref();
    let name = ILLEGAL_RE.replace_all(&name, "");
    let name = CONTROL_RE.replace_all(&name, "");
    let name = RESERVED_RE.replace(&name, "");

    let collect = |name: ::std::borrow::Cow<str>| {
        if name.len() > 255 {
            let mut end = 255;
            loop {
                if name.is_char_boundary(end) { break; }
                end -= 1;
            }
            String::from(&name[..end])
        } else {
            String::from(name)
        }
    };

    if options.windows {
        let name = WINDOWS_RESERVED_RE.replace(&name, "");
        let name = WINDOWS_TRAILING_RE.replace(&name, "");
        collect(name)
    } else {
        collect(name)
    }

}


#[cfg(test)]
mod tests {
    // From https://github.com/parshap/node-sanitize-filename/blob/master/test.js
    static NAMES: &'static [&'static str] = &[
        "the quick brown fox jumped over the lazy dog",
        "résumé",
        "hello\u{0000}world",
        "hello\nworld",
        "semi;colon.js",
        ";leading-semi.js",
        "slash\\.js",
        "slash/.js",
        "col:on.js",
        "star*.js",
        "question?.js",
        "quote\".js",
        "singlequote'.js",
        "brack<e>ts.js",
        "p|pes.js",
        "plus+.js",
        "'five and six<seven'.js",
        " space at front",
        "space at end ",
        ".period",
        "period.",
        "relative/path/to/some/dir",
        "/abs/path/to/some/dir",
        "~/.\u{0000}notssh/authorized_keys",
        "",
        "h?w",
        "h/w",
        "h*w",
        ".",
        "..",
        "./",
        "../",
        "/..",
        "/../",
        "*.|.",
        "./",
        "./foobar",
        "../foobar",
        "../../foobar",
        "./././foobar",
        "|*.what",
        "LPT9.asdf"
    ];

    static NAMES_CLEANED: &'static [&'static str] = &[
        "the quick brown fox jumped over the lazy dog",
        "résumé",
        "helloworld",
        "helloworld",
        "semi;colon.js",
        ";leading-semi.js",
        "slash.js",
        "slash.js",
        "colon.js",
        "star.js",
        "question.js",
        "quote.js",
        "singlequote'.js",
        "brackets.js",
        "ppes.js",
        "plus+.js",
        "'five and sixseven'.js",
        " space at front",
        "space at end ",
        ".period",
        "period.",
        "relativepathtosomedir",
        "abspathtosomedir",
        "~.notsshauthorized_keys",
        "",
        "hw",
        "hw",
        "hw",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        ".foobar",
        "..foobar",
        "....foobar",
        "...foobar",
        ".what",
        ""
    ];

    #[test]
    fn verify_sanitize() {
        let options = super::Options {
            windows: true
        };

        for (idx, name) in NAMES.iter().enumerate() {
            assert_eq!(super::sanitize_with_options(name, options.clone()), NAMES_CLEANED[idx]);
        }

        let long = ::std::iter::repeat('a').take(300).collect::<String>();
        let shorter = ::std::iter::repeat('a').take(255).collect::<String>();
        assert_eq!(super::sanitize(long), shorter);
    }
}
