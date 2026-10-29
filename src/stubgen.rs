//! `py2rs stub CRATE`: a stub (`CRATE.pyi`) from the crate's rustdoc JSON.
//!
//! Functions and inherent methods whose parameter and result types map to the
//! subset are declared; others are listed as skipped.

use serde_json::Value;

/// Python type (and Rust width) of a rustdoc type, or why not.
fn ty(t: &Value, self_name: &str, structs: &[String]) -> Result<(String, Option<String>, bool), String> {
    // (python type, width, is a reference)
    if let Some(p) = t.get("primitive").and_then(|p| p.as_str()) {
        return match p {
            "str" => Ok(("str".into(), None, false)),
            "bool" => Ok(("bool".into(), None, false)),
            "i64" => Ok(("int".into(), None, false)),
            "f64" => Ok(("float".into(), None, false)),
            "f32" => Ok(("float".into(), Some("f32".into()), false)),
            "usize" | "isize" | "u64" | "u32" | "u16" | "u8" | "i32" | "i16" | "i8" => Ok(("int".into(), Some(p.into()), false)),
            p => Err(format!("primitive {}", p)),
        };
    }
    if let Some(r) = t.get("borrowed_ref") {
        let (inner, w, _) = ty(&r["type"], self_name, structs)?;
        if r["is_mutable"].as_bool().unwrap_or(false) {
            return Ok((format!("Mut[{}]", inner), w, true));
        }
        return Ok((inner, w, true));
    }
    if let Some(g) = t.get("generic").and_then(|g| g.as_str()) {
        return if g == "Self" { Ok((format!("\"{}\"", self_name), None, false)) } else { Err(format!("generic {}", g)) };
    }
    if let Some(s) = t.get("slice") {
        let (inner, _, _) = ty(s, self_name, structs)?;
        return Ok((format!("list[{}]", inner), None, false));
    }
    if let Some(rp) = t.get("resolved_path") {
        let name = rp.get("path").or_else(|| rp.get("name")).and_then(|n| n.as_str()).unwrap_or("");
        let name = name.rsplit("::").next().unwrap_or(name);
        let args: Vec<&Value> = rp
            .pointer("/args/angle_bracketed/args")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|x| x.get("type")).collect())
            .unwrap_or_default();
        let arg = |i: usize| -> Result<String, String> {
            args.get(i).ok_or_else(|| format!("{} without arguments", name)).and_then(|a| ty(a, self_name, structs).map(|x| x.0))
        };
        return match name {
            "String" => Ok(("str".into(), None, false)),
            "Vec" => Ok((format!("list[{}]", arg(0)?), None, false)),
            "Option" => Ok((format!("{} | None", arg(0)?), None, false)),
            "Result" => Ok((format!("Result[{}]", arg(0)?), None, false)),
            n if n == self_name || structs.iter().any(|s| s == n) => Ok((format!("\"{}\"", n), None, false)),
            n => Err(format!("type {}", n)),
        };
    }
    Err("unsupported type".into())
}

fn func(item: &Value, self_name: Option<&str>, structs: &[String], out: &mut String, skipped: &mut Vec<String>, ind: &str) {
    let name = item["name"].as_str().unwrap_or("");
    let Some(f) = item.pointer("/inner/function") else { return };
    let has_generics = f.pointer("/generics/params").and_then(|p| p.as_array()).map(|p| !p.is_empty()).unwrap_or(false);
    if has_generics || item["visibility"].as_str() != Some("public") {
        if has_generics {
            skipped.push(format!("{} (generic)", name));
        }
        return;
    }
    let sn = self_name.unwrap_or("");
    let inputs = f.pointer("/sig/inputs").and_then(|i| i.as_array()).cloned().unwrap_or_default();
    let mut params = vec![];
    let mut has_self = false;
    let mut self_mut = false;
    for inp in &inputs {
        let pname = inp[0].as_str().unwrap_or("_");
        if pname == "self" {
            has_self = true;
            self_mut = inp[1].pointer("/borrowed_ref/is_mutable").and_then(|m| m.as_bool()).unwrap_or(false);
            if inp[1].get("borrowed_ref").is_none() {
                skipped.push(format!("{} (takes self by value)", name));
                return;
            }
            continue;
        }
        match ty(&inp[1], sn, structs) {
            Ok((t, w, _)) => params.push(format!("{}: {}", pname, w.unwrap_or(t))),
            Err(e) => {
                skipped.push(format!("{} ({})", name, e));
                return;
            }
        }
    }
    let (ret, conv) = match f.pointer("/sig/output") {
        None | Some(Value::Null) => ("None".to_string(), ""),
        Some(o) => match ty(o, sn, structs) {
            // a returned reference: owned copy for Python
            Ok((t, w, true)) => (w.unwrap_or(t.clone()), if t == "str" { ".to_string()" } else { ".clone()" }),
            Ok((t, w, false)) => (w.unwrap_or(t), ""),
            Err(e) => {
                skipped.push(format!("{} (returns {})", name, e));
                return;
            }
        },
    };
    let args: Vec<String> = (0..params.len()).map(|i| format!("{{{}}}", i)).collect();
    if !conv.is_empty() {
        let call = if has_self { format!("{{self}}.{}({}){}", name, args.join(", "), conv) } else { format!("{}::{}({}){}", sn, name, args.join(", "), conv) };
        *out += &format!("{}@rust({:?})\n", ind, call);
    }
    if self_name.is_some() && !has_self {
        *out += &format!("{}@staticmethod\n", ind);
    }
    if self_mut {
        *out += &format!("{}@mut\n", ind);
    }
    let mut all = vec![];
    if has_self {
        all.push("self".to_string());
    }
    all.extend(params);
    *out += &format!("{}def {}({}) -> {}: ...\n\n", ind, name, all.join(", "), ret);
}

/// Stub text from a rustdoc JSON document.
pub fn stub(krate: &str, dep: &str, doc: &Value) -> String {
    let index = &doc["index"];
    let root = doc["root"].to_string().trim_matches('"').to_string();
    let mut out = format!("# crate: {}\n\"\"\"py2rs stub for `{}`, generated by `py2rs stub` from rustdoc JSON.\"\"\"\n\nfrom rust import Mut, Result, rust, usize, isize, u64, u32, u16, u8, i32, i16, i8, f32\n\n", dep, krate);
    let mut skipped = vec![];
    let items = index[&root].pointer("/inner/module/items").and_then(|i| i.as_array()).cloned().unwrap_or_default();
    let get = |id: &Value| -> Option<&Value> { index.get(id.to_string().trim_matches('"')) };
    // the crate's own structs can appear in signatures
    let structs: Vec<String> = items
        .iter()
        .filter_map(|id| get(id))
        .filter(|i| i.pointer("/inner/struct").is_some())
        .filter_map(|i| i["name"].as_str().map(String::from))
        .collect();
    for id in &items {
        let Some(item) = get(id) else { continue };
        let name = item["name"].as_str().unwrap_or("");
        if let Some(st) = item.pointer("/inner/struct") {
            out += &format!("\nclass {}:\n", name);
            let mut body = String::new();
            for imp in st["impls"].as_array().cloned().unwrap_or_default() {
                let Some(imp) = get(&imp) else { continue };
                let inner = &imp["inner"]["impl"];
                if !inner["trait"].is_null() {
                    // Display gives __str__
                    if inner.pointer("/trait/path").or_else(|| inner.pointer("/trait/name")).and_then(|p| p.as_str()).map(|p| p.ends_with("Display")).unwrap_or(false) {
                        body += "    @rust(\"{self}.to_string()\")\n    def __str__(self) -> str: ...\n\n";
                    }
                    continue;
                }
                for m in inner["items"].as_array().cloned().unwrap_or_default() {
                    if let Some(mi) = get(&m) {
                        func(mi, Some(name), &structs, &mut body, &mut skipped, "    ");
                    }
                }
            }
            if body.is_empty() {
                body = "    pass\n".into();
            }
            out += &body;
        } else if item.pointer("/inner/function").is_some() {
            func(item, None, &structs, &mut out, &mut skipped, "");
        }
    }
    if !skipped.is_empty() {
        out += "\n# skipped (types outside the subset):\n";
        for s in skipped {
            out += &format!("#   {}\n", s);
        }
    }
    out
}
