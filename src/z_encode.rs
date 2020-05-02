pub fn z_encode(s: &str) -> Option<String> {
    let mut ret = String::with_capacity(s.len() * 2);
    let mut chars = s.chars();

    let mut next = chars.next();

    while let Some(c) = next {
        match c {
            '(' => {
                next = chars.next(); // consume '('
                let mut consumed = false;
                let mut unboxed = false;
                let mut arity = 0;
                loop {
                    match next {
                        Some('#') => {
                            unboxed = true;
                            next = chars.next(); // consume '#'
                            consumed = true;
                        }
                        Some(' ') => {
                            next = chars.next(); // consume ' '
                            consumed = true;
                        }
                        Some(')') => {
                            let kind = if unboxed { 'H' } else { 'T' };
                            ret.push_str(&format!("Z{}{}", arity, kind));
                            next = chars.next(); // consume ')'
                            break;
                        }
                        Some(',') => {
                            next = chars.next(); // consume ','
                            if arity == 0 {
                                arity = 2;
                            } else {
                                arity += 1;
                            }
                            while let Some(',') = next {
                                arity += 1;
                                next = chars.next(); // consume ','
                            }
                            consumed = true;
                        }
                        _ => {
                            if consumed {
                                return None;
                            } else {
                                ret.push_str("ZL");
                                break;
                            }
                        }
                    }
                }
            }
            ')' => {
                ret.push_str("ZR");
                next = chars.next();
            }
            '[' => {
                ret.push_str("ZM");
                next = chars.next();
            }
            ']' => {
                ret.push_str("ZN");
                next = chars.next();
            }
            ':' => {
                ret.push_str("ZC");
                next = chars.next();
            }
            '&' => {
                ret.push_str("za");
                next = chars.next();
            }
            '|' => {
                ret.push_str("zb");
                next = chars.next();
            }
            '^' => {
                ret.push_str("zc");
                next = chars.next();
            }
            '$' => {
                ret.push_str("zd");
                next = chars.next();
            }
            '=' => {
                ret.push_str("ze");
                next = chars.next();
            }
            '>' => {
                ret.push_str("zg");
                next = chars.next();
            }
            '#' => {
                ret.push_str("zh");
                next = chars.next();
            }
            '.' => {
                ret.push_str("zi");
                next = chars.next();
            }
            '<' => {
                ret.push_str("zl");
                next = chars.next();
            }
            '-' => {
                ret.push_str("zm");
                next = chars.next();
            }
            '!' => {
                ret.push_str("zn");
                next = chars.next();
            }
            '+' => {
                ret.push_str("zp");
                next = chars.next();
            }
            '\'' => {
                ret.push_str("zq");
                next = chars.next();
            }
            '\\' => {
                ret.push_str("zr");
                next = chars.next();
            }
            '/' => {
                ret.push_str("zs");
                next = chars.next();
            }
            '*' => {
                ret.push_str("zt");
                next = chars.next();
            }
            '_' => {
                ret.push_str("zu");
                next = chars.next();
            }
            '%' => {
                ret.push_str("zv");
                next = chars.next();
            }
            'z' => {
                ret.push_str("zz");
                next = chars.next();
            }
            'Z' => {
                ret.push_str("ZZ");
                next = chars.next();
            }
            c => {
                ret.push(c);
                next = chars.next();
            }
        }
    }

    debug_assert!(chars.next().is_none());
    Some(ret)
}

#[test]
fn encode_test() {
    assert_eq!(z_encode("("), Some("ZL".to_string()));
    assert_eq!(z_encode(")"), Some("ZR".to_string()));
    assert_eq!(z_encode("()"), Some("Z0T".to_string()));
    assert_eq!(z_encode("(# #)"), Some("Z0H".to_string()));
    assert_eq!(z_encode("(,)"), Some("Z2T".to_string()));
    assert_eq!(z_encode("(,,)"), Some("Z3T".to_string()));
    assert_eq!(z_encode("(#,#)"), Some("Z2H".to_string()));
    assert_eq!(z_encode("(#,,#)"), Some("Z3H".to_string()));
    assert_eq!(z_encode("Trak"), Some("Trak".to_string()));
    assert_eq!(z_encode("foo_wib"), Some("foozuwib".to_string()));
    assert_eq!(z_encode(">"), Some("zg".to_string()));
    assert_eq!(z_encode(">1"), Some("zg1".to_string()));
    assert_eq!(z_encode("foo#"), Some("foozh".to_string()));
    assert_eq!(z_encode("foo##"), Some("foozhzh".to_string()));
    assert_eq!(z_encode("foo##1"), Some("foozhzh1".to_string()));
    assert_eq!(z_encode("fooZ"), Some("fooZZ".to_string()));
    assert_eq!(z_encode(":+"), Some("ZCzp".to_string()));
}
