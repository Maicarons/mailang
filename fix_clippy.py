from pathlib import Path
p = Path("crates/mailang-analyzer/src/analyzer.rs")
c = p.read_text(encoding="utf-8")
old = "                if contains_return(inner) {`n                    return true;`n                }"
new = "                return contains_return(inner);"
if old in c:
    c = c.replace(old, new)
    print("replaced contains_return")
else:
    print("contains_return pattern not found")
old2 = "pub enum Type {"
new2 = "#[allow(clippy::enum_variant_names)]`npub enum Type {"
if old2 in c and "enum_variant_names" not in c:
    c = c.replace(old2, new2, 1)
    print("added enum allow")
p.write_text(c, encoding="utf-8")
