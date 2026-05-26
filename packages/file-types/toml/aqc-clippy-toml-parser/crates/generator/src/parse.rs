//! Parser for the upstream `define_Conf!` macro body.

#[derive(Debug, Clone)]
pub(crate) struct Field {
    pub(crate) name: String,
    pub(crate) rust_type: String,
    pub(crate) default: String,
    pub(crate) doc: String,
}

pub(crate) fn define_conf(source: &str) -> Vec<Field> {
    let mut fields = Vec::new();

    let macro_start = source
        .find("define_Conf!")
        .expect("define_Conf! macro not found");
    let brace_start = source[macro_start..]
        .find('{')
        .expect("opening brace not found")
        + macro_start;

    let mut depth = 1_i32;
    let mut pos = brace_start + 1;
    while depth > 0 && pos < source.len() {
        match source.as_bytes()[pos] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        pos += 1;
    }

    let macro_body = &source[brace_start + 1..pos - 1];
    let lines: Vec<&str> = macro_body.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        if line.is_empty() || line.starts_with("//") {
            i += 1;
            continue;
        }

        let mut doc = String::new();
        while i < lines.len() && lines[i].trim().starts_with("#[doc") {
            let doc_line = lines[i].trim();
            if let Some(start) = doc_line.find('"') {
                if let Some(end) = doc_line.rfind('"') {
                    if start < end {
                        doc.push_str(&doc_line[start + 1..end]);
                        doc.push(' ');
                    }
                }
            }
            i += 1;
        }

        while i < lines.len() && lines[i].trim().starts_with("#[") {
            i += 1;
        }

        if i >= lines.len() {
            break;
        }

        let field_line = lines[i].trim();

        if !field_line.contains(':') || !field_line.contains('=') {
            i += 1;
            continue;
        }

        let colon_pos = field_line.find(':').expect("colon checked above");
        let name = field_line[..colon_pos].trim().to_string();

        let after_colon = &field_line[colon_pos + 1..];
        if let Some(eq_pos) = after_colon.find('=') {
            let rust_type = after_colon[..eq_pos].trim().to_string();
            let default = after_colon[eq_pos + 1..]
                .trim()
                .trim_end_matches(',')
                .to_string();

            fields.push(Field {
                name,
                rust_type,
                default,
                doc: doc.trim().to_string(),
            });
        }

        i += 1;
    }

    fields
}
