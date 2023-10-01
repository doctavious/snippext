// based on https://github.com/dtolnay/indoc/tree/master/unindent but slightly modified

use std::iter::Peekable;
use std::slice::Split;

pub(crate) fn unindent(s: &str) -> String {
    let bytes = s.as_bytes();
    let unindented = unindent_bytes(bytes);
    String::from_utf8(unindented).unwrap()
}

// Compute the maximal number of spaces that can be removed from every line, and remove them.
pub(crate) fn unindent_bytes(s: &[u8]) -> Vec<u8> {
    // Document may start either on the same line as opening quote or on the next line
    let ignore_first_line = s.starts_with(b"\n") || s.starts_with(b"\r\n");

    // Largest number of spaces that can be removed from every
    // non-whitespace-only line after the first
    let spaces = s.lines().filter_map(count_spaces).min().unwrap_or(0);

    if spaces == 0 {
        return s.to_vec();
    }

    let mut result = Vec::with_capacity(s.len());
    for (i, line) in s.lines().enumerate() {
        if i > 1 || (i == 1 && !ignore_first_line) {
            result.push(b'\n');
        }
        if line.len() > spaces {
            // Whitespace-only lines may have fewer than the number of spaces being removed
            result.extend_from_slice(&line[spaces..]);
        }
    }
    result
}

pub(crate) trait Unindent {
    type Output;

    fn unindent(&self) -> Self::Output;
}

impl Unindent for str {
    type Output = String;

    fn unindent(&self) -> Self::Output {
        unindent(self)
    }
}

impl Unindent for String {
    type Output = String;

    fn unindent(&self) -> Self::Output {
        unindent(self)
    }
}

impl Unindent for [u8] {
    type Output = Vec<u8>;

    fn unindent(&self) -> Self::Output {
        unindent_bytes(self)
    }
}

impl<'a, T: ?Sized + Unindent> Unindent for &'a T {
    type Output = T::Output;

    fn unindent(&self) -> Self::Output {
        (**self).unindent()
    }
}

// Number of leading spaces in the line, or None if the line is entirely spaces.
pub(crate) fn count_spaces(line: &[u8]) -> Option<usize> {
    for (i, ch) in line.iter().enumerate() {
        if *ch != b' ' && *ch != b'\t' {
            return Some(i);
        }
    }
    None
}

pub(crate) fn count_spaces_string<S: AsRef<str>>(line: S) -> Option<usize> {
    count_spaces(line.as_ref().as_bytes())
}

// Based on core::str::StrExt.
trait BytesExt {
    fn lines(&self) -> Lines;
}

impl BytesExt for [u8] {
    fn lines(&self) -> Lines {
        fn is_newline(b: &u8) -> bool {
            *b == b'\n'
        }
        Lines {
            split: self.split(is_newline as fn(&u8) -> bool).peekable(),
        }
    }
}

struct Lines<'a> {
    split: Peekable<Split<'a, u8, fn(&u8) -> bool>>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.split.next()
    }
}
