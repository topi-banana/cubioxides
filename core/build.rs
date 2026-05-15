//! Generate `tables/btree*.rs` from the cubiomes-vendored `.h` files.
//!
//! Each input is a hand-formatted C header with three arrays —
//! `btreeN_steps`, `btreeN_param`, `btreeN_nodes` — plus an `enum {
//! btreeN_order = N };` line. The build script tokenises each array
//! and writes the values to `${OUT_DIR}/btreeN.rs` as Rust slice
//! literals (`pub const BTREE_N_STEPS: &[u32] = &[ ... ];`).

use std::env;
use std::fs;
use std::path::PathBuf;

const TABLES: &[&str] = &["btree18", "btree192", "btree19", "btree20", "btree21wd"];

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    for name in TABLES {
        let src = PathBuf::from(format!("tables-src/{name}.h"));
        println!("cargo:rerun-if-changed={}", src.display());
        let content =
            fs::read_to_string(&src).unwrap_or_else(|e| panic!("read {}: {e}", src.display()));
        let rust = transpile(name, &content);
        let dst = out_dir.join(format!("{name}.rs"));
        fs::write(&dst, rust).unwrap_or_else(|e| panic!("write {}: {e}", dst.display()));
    }
    println!("cargo:rerun-if-changed=build.rs");
}

fn transpile(name: &str, content: &str) -> String {
    use std::fmt::Write;

    let stripped = strip_c_comments(content);
    let order = extract_order(name, &stripped);
    let steps = extract_array(&stripped, &format!("{name}_steps"));
    let param = extract_array(&stripped, &format!("{name}_param"));
    let nodes = extract_array(&stripped, &format!("{name}_nodes"));

    let upper = name.to_uppercase();
    let mut s = String::with_capacity(nodes.len() * 24);
    writeln!(
        s,
        "// Auto-generated from `core/tables-src/{name}.h` by `core/build.rs`."
    )
    .unwrap();
    writeln!(s).unwrap();
    writeln!(s, "pub const {upper}_ORDER: u32 = {order};").unwrap();
    writeln!(s).unwrap();
    writeln!(
        s,
        "pub const {upper}_STEPS: &[u32] = &{}",
        format_u32_array(&steps)
    )
    .unwrap();
    writeln!(
        s,
        "pub const {upper}_PARAM: &[i32] = &{}",
        format_i32_array(&param)
    )
    .unwrap();
    writeln!(
        s,
        "pub const {upper}_NODES: &[u64] = &{}",
        format_u64_array(&nodes)
    )
    .unwrap();
    s
}

fn strip_c_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek().copied()) {
            ('/', Some('/')) => {
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut prev = '\0';
                for next in chars.by_ref() {
                    if prev == '*' && next == '/' {
                        break;
                    }
                    prev = next;
                }
            }
            _ => out.push(c),
        }
    }
    out
}

fn extract_order(name: &str, content: &str) -> u32 {
    let marker = format!("{name}_order");
    let pos = content
        .find(&marker)
        .unwrap_or_else(|| panic!("missing {marker}"));
    let tail = &content[pos + marker.len()..];
    // expects `... = 10 };` or similar
    let eq = tail.find('=').expect("expected `=` after order marker");
    let after_eq = &tail[eq + 1..];
    let end = after_eq
        .find(|c: char| !c.is_ascii_digit() && !c.is_whitespace())
        .expect("expected order to terminate");
    let num = after_eq[..end].trim();
    num.parse()
        .unwrap_or_else(|_| panic!("invalid order number {num:?} for {name}"))
}

/// Parse the body of `static const T name[] = { ... };` or `static
/// const T name[][2] = { ... };`. Returns the contained integers in
/// flat order. Hex literals (`0x...`) are recognised.
fn extract_array(content: &str, name: &str) -> Vec<i128> {
    let marker = format!("{name}[");
    let mut pos = content
        .find(&marker)
        .unwrap_or_else(|| panic!("missing array {name}"));
    pos += marker.len();
    // skip past the `]` / `[2]` etc. up to the first `{`.
    let open = content[pos..]
        .find('{')
        .unwrap_or_else(|| panic!("missing `{{` after {name}"))
        + pos;
    // find matching final `};`.
    let close = find_matching_brace(&content[open..]) + open;
    let body = &content[open + 1..close];
    let mut out = Vec::new();
    for token in body.split([',', '{', '}', '\n', '\r', '\t', ' ']) {
        let t = token.trim();
        if t.is_empty() {
            continue;
        }
        let value = if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
            i128::from_str_radix(hex, 16)
                .unwrap_or_else(|_| panic!("invalid hex literal {t:?} in {name}"))
        } else if let Some(neg) = t.strip_prefix('-') {
            -neg.parse::<i128>()
                .unwrap_or_else(|_| panic!("invalid integer {t:?} in {name}"))
        } else {
            t.parse::<i128>()
                .unwrap_or_else(|_| panic!("invalid integer {t:?} in {name}"))
        };
        out.push(value);
    }
    out
}

fn find_matching_brace(s: &str) -> usize {
    let mut depth: i32 = 0;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    panic!("unmatched `{{`");
}

fn format_u32_array(v: &[i128]) -> String {
    use std::fmt::Write;
    let mut s = String::from("[");
    for (i, &x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
            if i % 8 == 0 {
                s.push('\n');
            } else {
                s.push(' ');
            }
        }
        write!(s, "{}", x as u32).unwrap();
    }
    s.push_str("];\n");
    s
}

fn format_i32_array(v: &[i128]) -> String {
    use std::fmt::Write;
    let mut s = String::from("[");
    for (i, &x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
            if i % 8 == 0 {
                s.push('\n');
            } else {
                s.push(' ');
            }
        }
        write!(s, "{}", x as i32).unwrap();
    }
    s.push_str("];\n");
    s
}

fn format_u64_array(v: &[i128]) -> String {
    use std::fmt::Write;
    let mut s = String::from("[");
    for (i, &x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
            if i % 4 == 0 {
                s.push('\n');
            } else {
                s.push(' ');
            }
        }
        write!(s, "0x{:016x}", x as u64).unwrap();
    }
    s.push_str("];\n");
    s
}
