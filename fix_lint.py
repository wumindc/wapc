import re

with open('src/resources.rs', 'r') as f:
    content = f.read()

content = content.replace("normalize_path(&project.join", "normalize_path(project.join")
content = content.replace("normalize_path(&resource.origin_path)", "normalize_path(resource.origin_path.clone())")
content = content.replace("normalize_path(&instruction.origin_path)", "normalize_path(instruction.origin_path.clone())")

# For the strings we replaced, we might need to remove the clone if it expects AsRef<Path>.
# Actually, normalize_path takes `p: impl AsRef<std::path::Path>`.
# `&String` implements `AsRef<Path>`. `String` implements `AsRef<Path>`.
# So `normalize_path(&resource.origin_path)` is actually valid, but clippy says `the borrowed expression implements the required traits`.
# This is because `resource.origin_path` is a `String`. `String` implements `AsRef<Path>`.
# Therefore `normalize_path(resource.origin_path)` moves the string. But we don't need to move it.
# We can just use `normalize_path(&resource.origin_path)` and add `#[allow(clippy::needless_borrow)]` OR just `normalize_path(&resource.origin_path)` ...
# Wait, clippy says: `help: change this to: project.join(".vscode").join("mcp.json")` without `&`.
# Because `PathBuf` implements `AsRef<Path>`.
content = content.replace("normalize_path(&project.join", "normalize_path(project.join")
content = content.replace("normalize_path(&resource.origin_path)", "normalize_path(&resource.origin_path)") # Keep as is, wait...

with open('src/resources.rs', 'w') as f:
    f.write(content)
