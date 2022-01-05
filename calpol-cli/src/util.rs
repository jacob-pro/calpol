use std::fmt::{Debug, Display};
use std::io::{stdin, Read};
use std::str::FromStr;

use dialoguer::Password;

use crate::{CalpolError, ClientError};

pub fn read_input<T>(prompt: &str) -> Result<T, CalpolError>
where
    T: Clone + FromStr + Display,
    T::Err: Display + Debug,
{
    dialoguer::Input::new()
        .with_prompt(prompt)
        .interact_text()
        .map_err(|e| ClientError::DialoguerError(e).into())
}

pub fn read_password(use_stdin: bool) -> Result<String, CalpolError> {
    Ok(if use_stdin {
        let mut buf = String::new();
        stdin()
            .read_to_string(&mut buf)
            .map_err(|e| ClientError::FailedToReadStdin(e))?;
        buf
    } else {
        Password::new()
            .with_prompt("Enter Password")
            .interact()
            .map_err(|e| ClientError::DialoguerError(e))?
    })
}
