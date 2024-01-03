// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![deny(rust_2018_idioms)]

use base64::Engine;
use pyo3::PyTypeInfo;
use std::convert::TryInto;
use std::io::Write;
use subtle::ConstantTimeEq;

pub const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
    &base64::alphabet::BCRYPT,
    base64::engine::general_purpose::NO_PAD,
);

#[pyo3::prelude::pyfunction]
fn gensalt<'p>(
    py: pyo3::Python<'p>,
    rounds: Option<u16>,
    prefix: Option<&[u8]>,
) -> pyo3::PyResult<&'p pyo3::types::PyBytes> {
    let rounds = rounds.unwrap_or(12);
    let prefix = prefix.unwrap_or(b"2b");

    if prefix != b"2a" && prefix != b"2b" {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Supported prefixes are b'2a' or b'2b'",
        ));
    }

    if !(4..=31).contains(&rounds) {
        return Err(pyo3::exceptions::PyValueError::new_err("Invalid rounds"));
    }

    let mut salt = [0; 16];
    getrandom::getrandom(&mut salt).unwrap();

    let encoded_salt = BASE64_ENGINE.encode(salt);

    pyo3::types::PyBytes::new_with(
        py,
        1 + prefix.len() + 1 + 2 + 1 + encoded_salt.len(),
        |mut b| {
            write!(b, "$").unwrap();
            b.write_all(prefix).unwrap();
            write!(b, "$").unwrap();
            write!(b, "{:02.2}", rounds).unwrap();
            write!(b, "$").unwrap();
            b.write_all(encoded_salt.as_bytes()).unwrap();

            Ok(())
        },
    )
}

#[pyo3::prelude::pyfunction]
fn hashpw<'p>(
    py: pyo3::Python<'p>,
    password: &[u8],
    salt: &[u8],
) -> pyo3::PyResult<&'p pyo3::types::PyBytes> {
    // bcrypt originally suffered from a wraparound bug:
    // http://www.openwall.com/lists/oss-security/2012/01/02/4
    // This bug was corrected in the OpenBSD source by truncating inputs to 72
    // bytes on the updated prefix $2b$, but leaving $2a$ unchanged for
    // compatibility. However, pyca/bcrypt 2.0.0 *did* correctly truncate inputs
    // on $2a$, so we do it here to preserve compatibility with 2.0.0
    let password = &password[..password.len().min(72)];

    // salt here is not just the salt bytes, but rather an encoded value
    // containing a version number, number of rounds, and the salt.
    // Should be [prefix, cost, hash]. This logic is copied from `bcrypt`
    let raw_parts: Vec<_> = salt
        .split(|&b| b == b'$')
        .filter(|s| !s.is_empty())
        .collect();
    if raw_parts.len() != 3 {
        return Err(pyo3::exceptions::PyValueError::new_err("Invalid salt"));
    }
    let version = match raw_parts[0] {
        b"2y" => bcrypt::Version::TwoY,
        b"2b" => bcrypt::Version::TwoB,
        b"2a" => bcrypt::Version::TwoA,
        b"2x" => bcrypt::Version::TwoX,
        _ => {
            return Err(pyo3::exceptions::PyValueError::new_err("Invalid salt"));
        }
    };
    let cost = std::str::from_utf8(raw_parts[1])
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("Invalid salt"))?
        .parse::<u32>()
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("Invalid salt"))?;
    // The last component can contain either just the salt, or the salt and
    // the result hash, depending on if the `salt` value come from `hashpw` or
    // `gensalt`.
    let raw_salt = BASE64_ENGINE
        .decode(&raw_parts[2][..22])
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("Invalid salt"))?
        .try_into()
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("Invalid salt"))?;

    let hashed = py
        .allow_threads(|| bcrypt::hash_with_salt(password, cost, raw_salt))
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("Invalid salt"))?;
    Ok(pyo3::types::PyBytes::new(
        py,
        hashed.format_for_version(version).as_bytes(),
    ))
}

#[pyo3::prelude::pyfunction]
fn checkpw(py: pyo3::Python<'_>, password: &[u8], hashed_password: &[u8]) -> pyo3::PyResult<bool> {
    Ok(hashpw(py, password, hashed_password)?
        .as_bytes()
        .ct_eq(hashed_password)
        .into())
}

#[pyo3::prelude::pyfunction]
fn kdf<'p>(
    py: pyo3::Python<'p>,
    password: &[u8],
    salt: &[u8],
    desired_key_bytes: usize,
    rounds: u32,
    ignore_few_rounds: Option<bool>,
) -> pyo3::PyResult<&'p pyo3::types::PyBytes> {
    let ignore_few_rounds = ignore_few_rounds.unwrap_or(false);

    if password.is_empty() || salt.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "password and salt must not be empty",
        ));
    }

    if desired_key_bytes == 0 || desired_key_bytes > 512 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "desired_key_bytes must be 1-512",
        ));
    }

    if rounds < 1 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "rounds must be 1 or more",
        ));
    }

    pyo3::types::PyBytes::new_with(py, desired_key_bytes, |output| {
        py.allow_threads(|| {
            bcrypt_pbkdf::bcrypt_pbkdf(password, salt, rounds, output).unwrap();
        });
        Ok(())
    })
}

#[pyo3::prelude::pymodule]
fn _bcrypt(_py: pyo3::Python<'_>, m: &pyo3::types::PyModule) -> pyo3::PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(gensalt, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(hashpw, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(checkpw, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(kdf, m)?)?;

    m.add("__title__", "bcrypt")?;
    m.add(
        "__summary__",
        "Modern(-ish) password hashing for your software and your servers",
    )?;
    m.add("__uri__", "https://github.com/pyca/bcrypt/")?;

    // When updating this, also update pyproject.toml
    // This isn't named __version__ because passlib treats the existence of
    // that attribute as proof that we're a different module
    m.add("__version_ex__", "4.1.3")?;

    let author = "The Python Cryptographic Authority developers";
    m.add("__author__", author)?;
    m.add("__email__", "cryptography-dev@python.org")?;

    m.add("__license__", "Apache License, Version 2.0")?;
    m.add("__copyright__", format!("Copyright 2013-2024 {author}"))?;

    Ok(())
}
