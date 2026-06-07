import re

with open('src/resources.rs', 'r') as f:
    content = f.read()

# Add helper functions to tests module
helpers = """
    fn opencode_config_dir(home: &std::path::Path) -> std::path::PathBuf {
        let ctx = crate::platform_paths::PlatformPathContext::current_home_compatible(home);
        match ctx.platform {
            crate::platform_paths::PlatformKind::Macos => home.join(".config/opencode"),
            _ => ctx.config_dir.join("opencode"),
        }
    }

    fn normalize_path(p: impl AsRef<std::path::Path>) -> String {
        p.as_ref().display().to_string().replace('\\\\', "/")
    }
"""
content = content.replace("mod tests {", "mod tests {\n" + helpers)

# Replace opencode paths
content = content.replace('home.path().join(".config/opencode")', 'opencode_config_dir(home.path())')
content = content.replace('home.path().join(".config/opencode/opencode.json")', 'opencode_config_dir(home.path()).join("opencode.json")')
content = content.replace('home.path().join(".config/opencode/skills/git-release")', 'opencode_config_dir(home.path()).join("skills/git-release")')

# Replace assertions in VSCode MCP servers
content = re.sub(
    r'assert_eq!\(\s*resource\.origin_path,\s*project\.join\("\.vscode/mcp\.json"\)\.display\(\)\.to_string\(\)\s*\);',
    r'assert_eq!(normalize_path(&resource.origin_path), normalize_path(&project.join(".vscode").join("mcp.json")));',
    content
)

# Replace assertions in copilot instructions
content = re.sub(
    r'assert_eq!\(\s*instruction\.origin_path,\s*project\s*\.join\("\.github/copilot-instructions\.md"\)\s*\.display\(\)\s*\.to_string\(\)\s*\);',
    r'assert_eq!(normalize_path(&instruction.origin_path), normalize_path(&project.join(".github").join("copilot-instructions.md")));',
    content
)

with open('src/resources.rs', 'w') as f:
    f.write(content)
