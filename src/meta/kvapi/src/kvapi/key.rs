// Copyright 2021 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Defines kvapi::KVApi key behaviors.

use std::fmt::Debug;
use std::string::FromUtf8Error;

use crate::kvapi;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeyError {
    #[error(transparent)]
    FromUtf8Error(#[from] FromUtf8Error),

    #[error("Non-ascii char are not supported: '{non_ascii}'")]
    AsciiError { non_ascii: String },

    #[error("Expect {i}-th segment to be '{expect}', but: '{got}'")]
    InvalidSegment {
        i: usize,
        expect: String,
        got: String,
    },

    #[error("Expect {expect} segments, but: '{got}'")]
    WrongNumberOfSegments { expect: usize, got: String },

    #[error("Expect at least {expect} segments, but {actual} segments found")]
    AtleastSegments { expect: usize, actual: usize },

    #[error("Invalid id string: '{s}': {reason}")]
    InvalidId { s: String, reason: String },
}

/// Convert structured key to a string key used by kvapi::KVApi and backwards
pub trait Key: Debug
where Self: Sized
{
    const PREFIX: &'static str;

    /// Encode structured key into a string.
    fn to_string_key(&self) -> String;

    /// Decode str into a structured key.
    fn from_str_key(s: &str) -> Result<Self, kvapi::KeyError>;
}

impl kvapi::Key for String {
    const PREFIX: &'static str = "";

    fn to_string_key(&self) -> String {
        self.clone()
    }

    fn from_str_key(s: &str) -> Result<Self, kvapi::KeyError> {
        Ok(s.to_string())
    }
}

/// Function that escapes special characters in a string.
///
/// All characters except digit, alphabet and '_' are treated as special characters.
/// A special character will be converted into "%num" where num is the hexadecimal form of the character.
///
/// # Example
/// ```
/// let key = "data_bend!!";
/// let new_key = escape(&key);
/// assert_eq!("data_bend%21%21".to_string(), new_key);
/// ```
pub fn escape(key: &str) -> String {
    let mut new_key = Vec::with_capacity(key.len());

    fn hex(num: u8) -> u8 {
        match num {
            0..=9 => b'0' + num,
            10..=15 => b'a' + (num - 10),
            unreachable => unreachable!("Unreachable branch num = {}", unreachable),
        }
    }

    for char in key.as_bytes() {
        match char {
            b'0'..=b'9' => new_key.push(*char),
            b'_' | b'a'..=b'z' | b'A'..=b'Z' => new_key.push(*char),
            _other => {
                new_key.push(b'%');
                new_key.push(hex(*char / 16));
                new_key.push(hex(*char % 16));
            }
        }
    }

    // Safe unwrap(): there are no invalid utf char in it.
    String::from_utf8(new_key).unwrap()
}

/// The reverse function of escape_for_key.
///
/// # Example
/// ```
/// let key = "data_bend%21%21";
/// let original_key = unescape(&key);
/// assert_eq!(Ok("data_bend!!".to_string()), original_key);
/// ```
pub fn unescape(key: &str) -> Result<String, FromUtf8Error> {
    let mut new_key = Vec::with_capacity(key.len());

    fn unhex(num: u8) -> u8 {
        match num {
            b'0'..=b'9' => num - b'0',
            b'a'..=b'f' => num - b'a' + 10,
            unreachable => unreachable!("Unreachable branch num = {}", unreachable),
        }
    }

    let bytes = key.as_bytes();

    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                // The last byte of the string won't be '%'
                let mut num = unhex(bytes[index + 1]) * 16;
                num += unhex(bytes[index + 2]);
                new_key.push(num);
                index += 3;
            }
            other => {
                new_key.push(other);
                index += 1;
            }
        }
    }

    String::from_utf8(new_key)
}

/// Check if the `i`-th segment absent.
pub fn check_segment_absent(
    elt: Option<&str>,
    i: usize,
    encoded: &str,
) -> Result<(), kvapi::KeyError> {
    if elt.is_some() {
        Err(kvapi::KeyError::WrongNumberOfSegments {
            expect: i,
            got: encoded.to_string(),
        })
    } else {
        Ok(())
    }
}

/// Check if the `i`-th segment present.
pub fn check_segment_present<'a>(
    elt: Option<&'a str>,
    i: usize,
    key: &str,
) -> Result<&'a str, kvapi::KeyError> {
    if let Some(s) = elt {
        Ok(s)
    } else {
        Err(kvapi::KeyError::WrongNumberOfSegments {
            expect: i + 1,
            got: key.to_string(),
        })
    }
}

/// Check if the `i`-th segment equals `expect`.
pub fn check_segment(elt: &str, i: usize, expect: &str) -> Result<(), kvapi::KeyError> {
    if elt != expect {
        return Err(kvapi::KeyError::InvalidSegment {
            i,
            expect: expect.to_string(),
            got: elt.to_string(),
        });
    }
    Ok(())
}

/// Decode a string into u64 id.
pub fn decode_id(s: &str) -> Result<u64, kvapi::KeyError> {
    let id = s.parse::<u64>().map_err(|e| kvapi::KeyError::InvalidId {
        s: s.to_string(),
        reason: e.to_string(),
    })?;

    Ok(id)
}
