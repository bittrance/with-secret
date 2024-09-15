use anyhow::Result;
use nom::{
    branch::alt,
    bytes::complete::{escaped, is_a, is_not, tag, take_while},
    character::complete::{space0, space1},
    combinator::opt,
    multi::many1,
    sequence::{delimited, tuple},
    IResult,
};

use crate::WithError;

fn equal_sign(input: &str) -> IResult<&str, ()> {
    let (input, _) = space0(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, _) = space0(input)?;
    Ok((input, ()))
}

fn maybe_quoted_value(input: &str) -> IResult<&str, &str> {
    alt((
        delimited(
            tag("\""),
            escaped(is_not("\"\\"), '\\', is_a("\"")),
            tag("\""),
        ),
        delimited(tag("'"), escaped(is_not("'\\"), '\\', is_a("'")), tag("'")),
        is_not("'\" \r\n"),
    ))(input)
}

fn one_secret(input: &str) -> IResult<&str, (String, String)> {
    let (input, _) = take_while(|c: char| c.is_whitespace())(input)?;
    let (input, _) = opt(tuple((tag("export"), space1)))(input)?;
    let (input, (key, _, value)) = tuple((
        take_while(|c: char| c.is_alphanumeric() || c == '_'),
        equal_sign,
        maybe_quoted_value,
    ))(input)?;
    let (input, _) = opt(is_a("\r\n"))(input)?;
    let key = key.to_owned();
    let value = value
        .to_owned()
        .replace("\\\"", "\"")
        .replace("\\'", "'")
        .replace("\\\\", "\\");
    Ok((input, (key, value)))
}

pub fn parse_secrets(input: &str) -> Result<Vec<(String, String)>> {
    let res = many1(one_secret)(input);
    match res {
        Ok(("", secrets))  => Ok(secrets),
        Ok((tail, _)) => Err(WithError::ParseError(tail.to_owned()).into()),
        Err(err) => Err(WithError::ParseError(err.to_string()).into()),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{one_secret, parse_secrets};

    #[test]
    fn basic_kvpair() -> Result<()> {
        let (_, kvpair) = one_secret("FOO=bar")?;
        assert_eq!(kvpair, ("FOO".to_owned(), "bar".to_owned()));
        Ok(())
    }

    #[test]
    fn kvpair_with_leading_whitespace() -> Result<()> {
        let (_, kvpair) = one_secret(" FOO=bar")?;
        assert_eq!(kvpair, ("FOO".to_owned(), "bar".to_owned()));
        Ok(())
    }

    #[test]
    fn kvpair_with_quoted_value() -> Result<()> {
        let (_, kvpair) = one_secret("FOO=\"b\\\"ar\"")?;
        assert_eq!(kvpair, ("FOO".to_owned(), "b\"ar".to_owned()));
        let (_, kvpair) = one_secret("FOO='b\\'ar'")?;
        assert_eq!(kvpair, ("FOO".to_owned(), "b'ar".to_owned()));
        Ok(())
    }

    #[test]
    fn bash_exports() -> Result<()> {
        let (tail, kvpair) = one_secret("export FOO = \"bar\"\n")?;
        assert_eq!(kvpair, ("FOO".to_owned(), "bar".to_owned()));
        assert_eq!(tail, "");
        Ok(())
    }

    #[test]
    fn many_unquoted_secrets() -> Result<()> {
        let secrets = parse_secrets("FOO = bar\nBAZ = quux")?;
        assert_eq!(
            vec![
                ("FOO".to_owned(), "bar".to_owned()),
                ("BAZ".to_owned(), "quux".to_owned())
            ],
            secrets
        );
        Ok(())
    }

    #[test]
    fn many_quoted_secrets() -> Result<()> {
        let secrets = parse_secrets("export FOO = \"bar\"\nexport BAZ = \"quux\"")?;
        assert_eq!(
            vec![
                ("FOO".to_owned(), "bar".to_owned()),
                ("BAZ".to_owned(), "quux".to_owned())
            ],
            secrets
        );
        Ok(())
    }

    #[test]
    fn parse_error() {
        let res = parse_secrets("export FOO = \"bar\"\nexport ");
        assert!(matches!(res, Err(_)));
        let res = parse_secrets("export FOO = \"bar");
        assert!(matches!(res, Err(_)));
        let res = parse_secrets("FOO = bar baz");
        assert!(matches!(res, Err(_)));
    }
}
