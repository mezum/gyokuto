#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{FloatSuffix, IntSuffix};
    use logos::Logos;

    fn lit(src: &str) -> Result<Lit, &'static str> {
        let (token, span) = Token::lexer(src).spanned().next().unwrap();
        decode(token.unwrap(), &src[span])
    }

    fn int(value: u64, suffix: Option<IntSuffix>) -> Result<Lit, &'static str> {
        Ok(Lit::Int { value, suffix })
    }

    fn float(digits: &str, suffix: Option<FloatSuffix>) -> Result<Lit, &'static str> {
        Ok(Lit::Float {
            digits: digits.to_string(),
            suffix,
        })
    }

    #[test]
    fn integers() {
        assert_eq!(lit("0"), int(0, None));
        assert_eq!(lit("1_000"), int(1000, None));
        assert_eq!(lit("0xff"), int(255, None));
        assert_eq!(lit("0o17"), int(15, None));
        assert_eq!(lit("0b1010"), int(10, None));
        assert_eq!(lit("255u8"), int(255, Some(IntSuffix::U8)));
        assert_eq!(lit("0xffusize"), int(255, Some(IntSuffix::Usize)));
        assert_eq!(lit("1i64"), int(1, Some(IntSuffix::I64)));
        assert_eq!(lit("18446744073709551615"), int(u64::MAX, None));
        assert!(lit("18446744073709551616").is_err());
    }

    #[test]
    fn floats() {
        assert_eq!(lit("1_000.5"), float("1000.5", None));
        assert_eq!(lit("2.5E-3f32"), float("2.5E-3", Some(FloatSuffix::F32)));
        assert_eq!(lit("1f64"), float("1", Some(FloatSuffix::F64)));
    }
}
