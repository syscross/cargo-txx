use fancy_regex::Regex as FRegex;
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct TxxError {
    pub message: String,
}

/// 라인 단위로 `//` 이후를 지운다. 지운 자리는 공백으로 채워서
/// 원본과 줄 번호/컬럼이 어긋나지 않게 한다 (line_of 계산이 깨지지 않도록).
fn strip_line_comments(src: &str) -> String {
    src.lines()
        .map(|line| {
            if let Some(idx) = find_comment_start(line) {
                let mut s = line[..idx].to_string();
                s.push_str(&" ".repeat(line.len() - idx));
                s
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 아주 단순한 `//` 탐지 — 문자열 리터럴 안의 `//`는 아직 구분 못 함 (다음 한계).
fn find_comment_start(line: &str) -> Option<usize> {
    line.find("//")
}

pub fn check(src: &str) -> Result<(), Vec<TxxError>> {
    let clean = strip_line_comments(src);
    let structs = collect_structs(&clean);

    let mut errors = Vec::new();
    errors.extend(check_explicit_self(&clean, &structs));
    errors.extend(check_uninitialized_use(&clean, &structs));

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

type StructMap = HashMap<String, Vec<String>>;

fn collect_structs(src: &str) -> StructMap {
    let mut map = HashMap::new();
    let struct_re = Regex::new(r"struct\s+(\w+)\s*\{").unwrap();

    for cap in struct_re.captures_iter(src) {
        let name = cap[1].to_string();
        let open = cap.get(0).unwrap().end();
        if let Some(close) = find_matching_brace(src, open) {
            let body = &src[open..close];
            let field_re = Regex::new(r"(?m)^\s*(\w+)\s*:\s*[\w:<>]+\s*,").unwrap();
            let fields: Vec<String> = field_re
                .captures_iter(body)
                .map(|c| c[1].to_string())
                .collect();
            map.insert(name, fields);
        }
    }

    map
}

fn check_explicit_self(src: &str, structs: &StructMap) -> Vec<TxxError> {
    let mut errors = Vec::new();

    let all_fields: HashSet<String> = structs.values().flatten().cloned().collect();
    if all_fields.is_empty() {
        return errors;
    }

    let fn_body_re = Regex::new(r"fn\s+[\w:]+\s*\([^)]*\)\s*\{").unwrap();
    for m in fn_body_re.find_iter(src) {
        let body_start = m.end();
        if let Some(body_end) = find_matching_brace(src, body_start) {
            let body = &src[body_start..body_end];

            for field in &all_fields {
                let pattern = format!(r"(?<!self\.)(?<!\w)\b{field}\b");
                let bare_re = FRegex::new(&pattern).unwrap();
                for m in bare_re.find_iter(body) {
                    let m = m.unwrap();
                    let line = line_of(src, body_start + m.start());
                    errors.push(TxxError {
                        message: format!(
                            "error: bare field access `{field}` at line {line} — did you mean `self.{field}`?"
                        ),
                    });
                }
            }
        }
    }

    errors
}

fn check_uninitialized_use(src: &str, structs: &StructMap) -> Vec<TxxError> {
    let mut errors = Vec::new();

    let let_re = Regex::new(r"\blet\s+(\w+)\s*:\s*(\w+)\s*;").unwrap();

    for cap in let_re.captures_iter(src) {
        let var = &cap[1];
        let type_name = &cap[2];
        let decl_end = cap.get(0).unwrap().end();

        let Some(fields) = structs.get(type_name) else {
            continue;
        };
        if fields.is_empty() {
            continue;
        }

        let rest = &src[decl_end..];

        let assign_re = Regex::new(&format!(r"\b{var}\.(\w+)\s*=")).unwrap();
        let call_re = Regex::new(&format!(r"\b{var}\.(\w+)\s*\(")).unwrap();

        let mut events: Vec<(usize, bool, String)> = Vec::new();
        for c in assign_re.captures_iter(rest) {
            events.push((c.get(0).unwrap().start(), true, c[1].to_string()));
        }
        for c in call_re.captures_iter(rest) {
            events.push((c.get(0).unwrap().start(), false, c[1].to_string()));
        }
        events.sort_by_key(|e| e.0);

        let mut assigned: HashSet<String> = HashSet::new();
        for (pos, is_assign, name) in events {
            if is_assign {
                assigned.insert(name);
            } else {
                let missing: Vec<&String> =
                    fields.iter().filter(|f| !assigned.contains(*f)).collect();
                if !missing.is_empty() {
                    let line = line_of(src, decl_end + pos);
                    let missing_str = missing
                        .iter()
                        .map(|s| format!("`{s}`"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    errors.push(TxxError {
                        message: format!(
                            "error: use of possibly-uninitialized value `{var}` at line {line} — missing field(s): {missing_str}"
                        ),
                    });
                }
                break;
            }
        }
    }

    errors
}

pub fn transpile(src: &str) -> String {
    // 검사는 주석 제거본으로 했지만, 변환은 원본 기준으로 해서
    // 최종 .cpp에도 주석이 그대로 살아있게 한다.
    let mut out = src.to_string();

    out = out.replace("import std;", "#include <iostream>\n#include <string>");
    out = Regex::new(r"\bself\.").unwrap().replace_all(&out, "this->").to_string();

    let field_re = Regex::new(r"(?m)^(\s*)(\w+)\s*:\s*([\w:<>]+)\s*,\s*$").unwrap();
    out = field_re.replace_all(&out, "$1$3 $2;").to_string();

    let let_re = Regex::new(r"\blet\s+(\w+)\s*:\s*([\w:<>]+)\s*;").unwrap();
    out = let_re.replace_all(&out, "$2 $1;").to_string();

    out = insert_method_declarations(&out);

    let fn_main_re = Regex::new(r"\bfn(\s+main\s*\()").unwrap();
    out = fn_main_re.replace_all(&out, "int$1").to_string();
    let fn_other_re = Regex::new(r"\bfn\b").unwrap();
    out = fn_other_re.replace_all(&out, "auto").to_string();

    transpile_braces(&out)
}

fn insert_method_declarations(src: &str) -> String {
    let method_re = Regex::new(r"fn\s+(\w+)::(\w+)\s*\(([^)]*)\)").unwrap();

    let mut decls: HashMap<String, Vec<String>> = HashMap::new();
    for cap in method_re.captures_iter(src) {
        let type_name = &cap[1];
        let method_name = &cap[2];
        let params = &cap[3];
        decls
            .entry(type_name.to_string())
            .or_default()
            .push(format!("auto {method_name}({params});"));
    }

    let mut out = src.to_string();
    for (type_name, methods) in decls {
        let struct_open_re = Regex::new(&format!(r"struct\s+{type_name}\s*\{{")).unwrap();
        if let Some(m) = struct_open_re.find(&out) {
            let insert_at = m.end();
            let insertion: String =
                methods.iter().map(|d| format!("\n    {d}")).collect();
            out.insert_str(insert_at, &insertion);
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq)]
enum BlockKind {
    ClassLike,
    MainFn,
    Other,
}

fn transpile_braces(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut result = String::new();
    let mut stack: Vec<(BlockKind, usize)> = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '{' {
            let tail = result.trim_end();
            let kind = if tail.ends_with(')') && tail.contains("int main(") {
                BlockKind::MainFn
            } else if ends_with_class_or_struct_decl(tail) {
                BlockKind::ClassLike
            } else {
                BlockKind::Other
            };
            result.push(c);
            stack.push((kind, result.len()));
            i += 1;
            continue;
        }

        if c == '}' {
            if let Some((kind, body_start)) = stack.pop() {
                match kind {
                    BlockKind::MainFn => {
                        let body = insert_line_semicolons(&result[body_start..]);
                        result.truncate(body_start);
                        result.push_str(&body);
                        if !body.contains("return") {
                            result.push_str("\n    return 0;\n");
                        }
                        result.push(c);
                    }
                    BlockKind::Other => {
                        let body = insert_line_semicolons(&result[body_start..]);
                        result.truncate(body_start);
                        result.push_str(&body);
                        result.push(c);
                    }
                    BlockKind::ClassLike => {
                        result.push(c);
                        result.push(';');
                    }
                }
            } else {
                result.push(c);
            }
            i += 1;
            continue;
        }

        result.push(c);
        i += 1;
    }

    result
}

/// 세미콜론 자동 삽입 시에도 주석 줄은 건드리지 않는다.
fn insert_line_semicolons(body: &str) -> String {
    body.lines()
        .map(|line| {
            let trimmed = line.trim_end();
            let is_comment = trimmed.trim_start().starts_with("//");
            let needs_semi = !trimmed.is_empty()
                && !is_comment
                && !trimmed.ends_with(';')
                && !trimmed.ends_with('{')
                && !trimmed.ends_with('}');
            if needs_semi {
                format!("{trimmed};")
            } else {
                trimmed.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn ends_with_class_or_struct_decl(tail: &str) -> bool {
    let re = Regex::new(r"\b(class|struct)\s+\w+\s*$").unwrap();
    re.is_match(tail)
}

fn find_matching_brace(src: &str, open_pos: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let mut depth = 1;
    let mut i = open_pos;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn line_of(src: &str, byte_pos: usize) -> usize {
    src[..byte_pos].matches('\n').count() + 1
}