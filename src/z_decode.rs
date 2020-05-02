use std::convert::TryFrom;

pub fn z_decode(s: &str) -> Option<String> {
    let mut ret = String::with_capacity(s.len());
    let mut chars = s.chars();

    let mut next = chars.next();

    while let Some(c) = next {
        match c {
            'z' => {
                next = chars.next(); // consume 'z'
                match next {
                    Some('a') => {
                        ret.push('&');
                    }
                    Some('b') => {
                        ret.push('|');
                    }
                    Some('c') => {
                        ret.push('^');
                    }
                    Some('d') => {
                        ret.push('$');
                    }
                    Some('e') => {
                        ret.push('=');
                    }
                    Some('g') => {
                        ret.push('>');
                    }
                    Some('h') => {
                        ret.push('#');
                    }
                    Some('i') => {
                        ret.push('.');
                    }
                    Some('l') => {
                        ret.push('<');
                    }
                    Some('m') => {
                        ret.push('-');
                    }
                    Some('n') => {
                        ret.push('!');
                    }
                    Some('p') => {
                        ret.push('+');
                    }
                    Some('q') => {
                        ret.push('\'');
                    }
                    Some('r') => {
                        ret.push('\\');
                    }
                    Some('s') => {
                        ret.push('/');
                    }
                    Some('t') => {
                        ret.push('*');
                    }
                    Some('u') => {
                        ret.push('_');
                    }
                    Some('v') => {
                        ret.push('%');
                    }
                    Some('z') => {
                        ret.push('z');
                    }
                    Some(c) if c >= '0' && c <= '9' => {
                        // Read hex
                        let mut hex = String::new();
                        if c != '0' {
                            hex.push(c);
                        }
                        next = chars.next(); // consume first char
                        loop {
                            match next {
                                Some('U') => {
                                    // FIXME: It's unclear what the encoding is, both in GHC's
                                    // z-encoding and Rust's TryFrom<u32> for char .........
                                    // but this seems to work fine for ASCII chars
                                    let num = u32::from_str_radix(&hex, 16).unwrap();
                                    match char::try_from(num) {
                                        Ok(char) => {
                                            ret.push(char);
                                            break;
                                        }
                                        Err(_) => {
                                            return None;
                                        }
                                    }
                                }
                                Some(c) if c.is_digit(16) => {
                                    hex.push(c);
                                    next = chars.next();
                                }
                                _ => {
                                    return None;
                                }
                            }
                        }
                    }
                    _ => {
                        return None;
                    }
                }
                next = chars.next();
            }
            'Z' => {
                next = chars.next(); // consume 'Z'
                match next {
                    Some('Z') => {
                        ret.push('Z');
                        next = chars.next(); // consume 'Z'
                    }
                    Some('L') => {
                        ret.push('(');
                        next = chars.next(); // consume 'L'
                    }
                    Some('R') => {
                        ret.push(')');
                        next = chars.next();
                    }
                    Some('M') => {
                        ret.push('[');
                        next = chars.next();
                    }
                    Some('N') => {
                        ret.push(']');
                        next = chars.next();
                    }
                    Some('C') => {
                        ret.push(':');
                        next = chars.next();
                    }
                    Some(c) if c.is_digit(10) => {
                        let mut num_str = String::new();
                        let mut unboxed = false;
                        num_str.push(c);
                        loop {
                            next = chars.next(); // consume digit char
                            match next {
                                Some('H') => {
                                    unboxed = true;
                                    break;
                                }
                                Some('T') => {
                                    break;
                                }
                                Some(c) if c.is_digit(10) => {
                                    num_str.push(c);
                                }
                                _ => {
                                    return None;
                                }
                            }
                        }
                        next = chars.next(); // consume 'H' or 'T'
                        match num_str.parse::<u8>() {
                            Ok(num) => {
                                if unboxed {
                                    ret.push_str("(#");
                                } else {
                                    ret.push('(');
                                }
                                if num == 0 {
                                    if unboxed {
                                        ret.push_str(" #)");
                                    } else {
                                        ret.push(')');
                                    }
                                } else {
                                    for _ in 0..num - 1 {
                                        ret.push(',');
                                    }
                                    if unboxed {
                                        ret.push('#');
                                    }
                                    ret.push(')');
                                }
                            }
                            Err(_) => {
                                return None;
                            }
                        }
                    }
                    _ => {
                        return None;
                    }
                }
            }
            c => {
                next = chars.next();
                ret.push(c);
            }
        }
    }

    debug_assert!(chars.next().is_none());
    Some(ret)
}

#[test]
fn decode_test() {
    assert_eq!(z_decode("ZL"), Some("(".to_string()));
    assert_eq!(z_decode("ZR"), Some(")".to_string()));
    assert_eq!(z_decode("Z0T"), Some("()".to_string()));
    assert_eq!(z_decode("Z0H"), Some("(# #)".to_string()));
    assert_eq!(z_decode("Z2T"), Some("(,)".to_string()));
    assert_eq!(z_decode("Z3T"), Some("(,,)".to_string()));
    assert_eq!(z_decode("Z2H"), Some("(#,#)".to_string()));
    assert_eq!(z_decode("Z3H"), Some("(#,,#)".to_string()));
    assert_eq!(z_decode("Trak"), Some("Trak".to_string()));
    assert_eq!(z_decode("foozuwib"), Some("foo_wib".to_string()));
    assert_eq!(z_decode("zg"), Some(">".to_string()));
    assert_eq!(z_decode("zg1"), Some(">1".to_string()));
    assert_eq!(z_decode("foozh"), Some("foo#".to_string()));
    assert_eq!(z_decode("foozhzh"), Some("foo##".to_string()));
    assert_eq!(z_decode("foozhzh1"), Some("foo##1".to_string()));
    assert_eq!(z_decode("fooZZ"), Some("fooZ".to_string()));
    assert_eq!(z_decode("ZCzp"), Some(":+".to_string()));
    assert_eq!(z_decode("z2cU"), Some(",".to_string()));
}
