// from https://github.com/kardeiz/sanitize-filename

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

pub(crate) fn sanitize<S: AsRef<str>>(name: S) -> String {

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

    if cfg!(windows) {
        let name = WINDOWS_RESERVED_RE.replace(&name, "");
        let name = WINDOWS_TRAILING_RE.replace(&name, "");
        collect(name)
    } else {
        collect(name)
    }

}
