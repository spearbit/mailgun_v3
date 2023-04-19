//! `reqwest` based web bindings for Mailgun's [v3 JSON API](https://documentation.mailgun.com/en/latest/api_reference.html)
//!
//! This crate wraps some of Mailgun's APIs, but doesn't attempt to do much else
//! in terms of error handling or argument sanitization

extern crate chrono;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;

pub mod email;
pub mod validation;

use lazy_static::lazy_static;
use regex::Regex;
use std::fmt;

pub use reqwest::Error as ReqError;

const MAILGUN_DEFAULT_API: &str = "https://api.mailgun.net/v3";

///! Wrapper result type returning `reqwest` errors
pub type MailgunResult<T> = Result<T, ReqError>;

///! Mailgun private API key and sending domain
#[derive(Debug, Clone)]
pub struct Credentials {
    api_base: String,
    api_key: String,
    domain: String,
}

impl Credentials {
    pub fn new<A: AsRef<str>, D: AsRef<str>>(api_key: A, domain: D) -> Self {
        Self::with_base(MAILGUN_DEFAULT_API, api_key, domain)
    }
    pub fn with_base<B: AsRef<str>, A: AsRef<str>, D: AsRef<str>>(
        api_base: B,
        api_key: A,
        domain: D,
    ) -> Self {
        let api_base = api_base.as_ref();
        let api_key = api_key.as_ref();
        let domain = domain.as_ref();
        assert!(
            api_base.starts_with("http"),
            "Domain does not start with http"
        );
        assert!(
            api_base.chars().filter(|c| *c == '.').count() >= 1,
            "api_base does not contain any dots"
        );
        assert!(api_key.len() >= 35, "api_key is to short");
        assert!(
            domain.chars().filter(|c| *c == '.').count() >= 1,
            "Domain does not contain any dots"
        );
        Credentials {
            api_base: api_base.to_string(),
            api_key: api_key.to_string(),
            domain: domain.to_string(),
        }
    }
    pub fn domain(&self) -> &str {
        &self.domain
    }
}

///! An email address, with or without a display name
#[derive(Debug, Clone, PartialEq)]
pub struct EmailAddress {
    name: Option<String>,
    address: String,
}

// TODO: introduce address validation (RFC5322 + RFC5198 + RFC6532)
// Could consider using the email-address-parser crate (or similar).

impl EmailAddress {
    pub fn address<T: ToString>(address: T) -> Self {
        EmailAddress {
            name: None,
            address: address.to_string(),
        }
    }

    pub fn name_address<T: ToString>(name: T, address: T) -> Self {
        EmailAddress {
            name: Some(name.to_string()),
            address: address.to_string(),
        }
    }

    pub fn email(&self) -> &str {
        &self.address
    }
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.name {
            Some(ref name) => write!(f, "{} <{}>", name, self.address),
            None => write!(f, "{}", self.address.clone()),
        }
    }
}

/// Basic validation of display name.
fn is_valid_display_name(name: &str) -> bool {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^[^<>]+$").unwrap();
    }
    RE.is_match(name)
}

/// Basic validation of address.
fn is_valid_address(address: &str) -> bool {
    lazy_static! {
        // TODO: use a proper regex
        static ref RE: Regex = Regex::new(r"^[^<> ]+@[^<> ]+\.[^<> ]+$").unwrap();
    }
    RE.is_match(address)
}

impl<'a> TryFrom<&'a str> for EmailAddress {
    type Error = &'static str;

    /// This parser does not validate the emails, just tries to parse according to
    /// a minimal subset of the RFC5322 rules.
    fn try_from(input: &str) -> Result<EmailAddress, Self::Error> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^(.*) <([^>]+)>$").unwrap();
        }

        let result = match RE.captures(input) {
            Some(captures) => {
                if captures.len() == 3 {
                    EmailAddress::name_address(
                        captures.get(1).unwrap().as_str(),
                        captures.get(2).unwrap().as_str(),
                    )
                } else {
                    EmailAddress::address(input)
                }
            }
            None => EmailAddress::address(input),
        };

        if let Some(ref name) = result.name {
            if !is_valid_display_name(name) {
                return Err("Invalid display name");
            }
        }

        if !is_valid_address(&result.address) {
            Err("Invalid email address")
        } else {
            Ok(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_email_address() {
        let success_cases = vec![
            ("test@email.com", EmailAddress::address("test@email.com")),
            (
                "Bob Test <test@email.com>",
                EmailAddress::name_address("Bob Test", "test@email.com"),
            ),
        ];
        for (input, expected) in success_cases {
            let result = EmailAddress::try_from(input);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }

        let failure_cases = vec![
            ("test", "Invalid email address"),
            ("@email.com", "Invalid email address"),
            ("Bob Test", "Invalid email address"),
            ("Bob Test <>", "Invalid email address"),
            ("Bob Test <test>", "Invalid email address"),
            ("Bob Test <@email.com>", "Invalid email address"),
            ("<Bob Test> <test@email.com>", "Invalid display name"),
        ];
        for (input, expected) in failure_cases {
            let result = EmailAddress::try_from(input);
            assert!(result.is_err());
            assert_eq!(result.err(), Some(expected));
        }
    }
}
