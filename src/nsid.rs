use std::{fmt, str::FromStr};

use crate::{
    error::ParseError, is_valid_domain_segment, is_valid_nsid_name, is_valid_tld, SEGMENT_LEN_RANGE,
};

const MAX_LEN: usize = 317;
const MAX_AUTHORITY_LEN: usize = 253;
const MIN_SEGMENTS: usize = 3;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nsid(String);

impl Nsid {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn segments(&self) -> impl DoubleEndedIterator<Item = &str> {
        self.0.split('.')
    }
}

impl fmt::Display for Nsid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl FromStr for Nsid {
    type Err = ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate_nsid(s.as_bytes()).map(|()| Nsid(s.into()))
    }
}

impl TryFrom<&'_ [u8]> for Nsid {
    type Error = ParseError;

    #[inline]
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        validate_nsid(bytes).map(|()| Nsid(String::from_utf8(bytes.into()).unwrap()))
    }
}

fn validate_nsid(bytes: &[u8]) -> Result<(), ParseError> {
    if bytes.len() > MAX_LEN {
        return Err(ParseError::nsid());
    }

    let mut it = bytes.split(|&b| b == b'.').peekable();

    let tld = it.next().ok_or_else(ParseError::nsid)?;

    if !is_valid_tld(tld) {
        return Err(ParseError::nsid());
    }

    let mut len = tld.len();
    let mut num_segments = 1;
    while let Some(segment) = it.next() {
        let is_valid = match it.peek() {
            Some(_) => is_valid_domain_segment(segment),
            None => len < MAX_AUTHORITY_LEN && is_valid_nsid_name(segment),
        };

        num_segments += 1;
        len += 1 + segment.len();

        if !is_valid {
            return Err(ParseError::nsid());
        }
    }

    if num_segments < MIN_SEGMENTS {
        return Err(ParseError::nsid());
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fragment(String);

impl Fragment {
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for Fragment {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate_fragment(s.as_bytes()).map(|()| Fragment(s.into()))
    }
}

fn validate_fragment(bytes: &[u8]) -> Result<(), ParseError> {
    let bytes = bytes
        .strip_prefix(b"#")
        .ok_or_else(ParseError::nsid_fragment)?;

    if !SEGMENT_LEN_RANGE.contains(&bytes.len()) {
        return Err(ParseError::nsid_fragment());
    }

    if !bytes.iter().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ParseError::nsid_fragment());
    }

    Ok(())
}

impl fmt::Display for Fragment {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// An NSID reference.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Reference {
    Full(FullReference),
    Relative(Fragment),
}

impl FromStr for Reference {
    type Err = ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('#') {
            Fragment::from_str(s).map(Reference::Relative)
        } else {
            FullReference::from_str(s).map(Reference::Full)
        }
    }
}

impl fmt::Display for Reference {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reference::Full(r) => fmt::Display::fmt(r, f),
            Reference::Relative(r) => fmt::Display::fmt(r, f),
        }
    }
}

/// A fully-qualified NSID reference.
///
/// This consists of an NSID and an optional fragment.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FullReference {
    text: String,
    frag_start: usize,
}

impl FullReference {
    pub fn clone_nsid(&self) -> Nsid {
        Nsid(self.text[..self.frag_start].to_string())
    }

    #[inline]
    fn has_fragment(&self) -> bool {
        (self.frag_start < self.text.len())
    }

    pub fn clone_fragment(&self) -> Option<Fragment> {
        self.has_fragment()
            .then(|| Fragment(self.text[self.frag_start..].to_string()))
    }

    pub fn fragment_name(&self) -> Option<&str> {
        self.has_fragment()
            .then_some(&self.text[self.frag_start + 1..])
    }
}

impl From<Nsid> for FullReference {
    #[inline]
    fn from(nsid: Nsid) -> Self {
        FullReference {
            frag_start: nsid.0.len(),
            text: nsid.0,
        }
    }
}

impl FromStr for FullReference {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let frag_start = s.find('#').unwrap_or(s.len());
        let (nsid_s, frag_s) = s.split_at(frag_start);

        validate_nsid(nsid_s.as_bytes())?;

        if !frag_s.is_empty() {
            validate_fragment(frag_s.as_bytes())?;
        }

        Ok(FullReference {
            text: s.into(),
            frag_start,
        })
    }
}

impl fmt::Display for FullReference {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_examples() {
        crate::test::test_valid::<Nsid>([
            "com.example.fooBar",
            "net.users.bob.ping",
            "a-0.b-1.c",
            "a.b.c",
            "cn.8.lex.stuff",
        ]);
    }

    #[test]
    fn invalid_examples() {
        crate::test::test_invalid::<Nsid>(["com.exa🤯ple.thing", "com.example"]);
    }
}
