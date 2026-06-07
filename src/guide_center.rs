//! Built-in safe usage guides for resource details.
//! @author codex

use crate::model::{ResourceGuide, ResourceGuideSection};

const UPDATED_AT: &str = "2026-06-06T00:00:00Z";

pub fn get_resource_guide(
    tool: Option<&str>,
    kind: Option<&str>,
    resource_id: Option<&str>,
) -> ResourceGuide {
    let kind = kind
        .map(str::to_string)
        .or_else(|| resource_id.and_then(|id| id.split(':').next().map(str::to_string)))
        .unwrap_or_else(|| "resource".to_string());
    let tool = tool.map(str::to_string);
    let title = match (&tool, kind.as_str()) {
        (Some(tool), "mcp") => format!("{} MCP 使用说明", display_tool(tool)),
        (Some(tool), "instruction") => format!("{} 指令文件使用说明", display_tool(tool)),
        (Some(tool), "skill") => format!("{} Skills 使用说明", display_tool(tool)),
        (Some(tool), "plugin") => format!("{} Plugins 使用说明", display_tool(tool)),
        (Some(tool), "subagent") => format!("{} Subagents 使用说明", display_tool(tool)),
        (_, "mcp") => "MCP 使用说明".to_string(),
        (_, "instruction") => "指令文件使用说明".to_string(),
        (_, "skill") => "Skills 使用说明".to_string(),
        (_, "plugin") => "Plugins 使用说明".to_string(),
        (_, "subagent") => "Subagents 使用说明".to_string(),
        _ => "资源使用说明".to_string(),
    };

    ResourceGuide {
        id: guide_id(tool.as_deref(), &kind),
        tool,
        kind: kind.clone(),
        title,
        summary: summary_for_kind(&kind).to_string(),
        sections: sections_for_kind(&kind),
        risks: vec![
            "备份会保存目标工具原配置，可能包含该文件中已有的密钥值。".to_string(),
            "跨 Scope 或跨工具写入必须经过显式授权、预览和可回滚记录。".to_string(),
        ],
        unsupported_actions: vec![
            "enterprise 或 managed 范围资源保持只读，不生成写入计划。".to_string(),
            "插件提供的资源由插件机制管理，当前不开放直接写入。".to_string(),
        ],
        updated_at: UPDATED_AT.to_string(),
    }
}

fn guide_id(tool: Option<&str>, kind: &str) -> String {
    match tool {
        Some(tool) => format!("guide:{tool}:{kind}"),
        None => format!("guide:any:{kind}"),
    }
}

fn display_tool(tool: &str) -> &'static str {
    match tool {
        "claude" => "Claude Code",
        "codex" => "Codex",
        "gemini" => "Gemini CLI",
        "cursor" => "Cursor",
        "opencode" => "OpenCode",
        _ => "AI 编程工具",
    }
}

fn summary_for_kind(kind: &str) -> &'static str {
    match kind {
        "mcp" => "说明 MCP 资源的用途、配置要点、可管理边界和写入风险。",
        "instruction" => "说明指令文件如何影响工具行为，以及 WAPC 的结构化只读边界。",
        "skill" => "说明 Skills 的目录结构、识别方式和当前只读管理边界。",
        "plugin" => "说明 Plugins 的组件关系、所有权边界和当前只读策略。",
        "subagent" => "说明 Subagents 的 frontmatter 元数据和正文隐私边界。",
        _ => "说明资源在 WAPC 中的识别、展示和安全操作边界。",
    }
}

fn sections_for_kind(kind: &str) -> Vec<ResourceGuideSection> {
    match kind {
        "mcp" => vec![
            section(
                "用途",
                "MCP 让工具连接外部服务或本地能力。WAPC 只展示名称、transport、命令摘要、URL、env key 名称和指纹等脱敏元数据。",
            ),
            section(
                "配置要点",
                "JSON/TOML 写入必须由 Sync Engine 生成预览，确认后执行备份、原子写入、重新读取校验和失败回滚。",
            ),
            section(
                "安全提醒",
                "不要把密钥正文写入模板、同步预设、变更日志或数据库；需要密钥时只允许沿用目标已有值或由用户当次手填。",
            ),
        ],
        "instruction" => vec![
            section(
                "用途",
                "指令文件用于约束工具行为。WAPC 只保存标题、段落指纹、字节数和内容哈希，不保存正文。",
            ),
            section(
                "安全提醒",
                "编辑指令正文属于写入能力，必须先通过 Sync Engine 和备份校验链路；当前说明只做关联展示。",
            ),
        ],
        "skill" => vec![
            section(
                "用途",
                "Skills 通常包含说明、脚本和模板。WAPC 只盘点文件清单、大小和聚合哈希。",
            ),
            section(
                "安全提醒",
                "删除或改动 Skill 目录前必须能备份并回滚；未实现完整机制前保持只读。",
            ),
        ],
        "plugin" => vec![
            section(
                "用途",
                "Plugins 可能提供 skills、MCP、apps 或其他扩展。WAPC 展示组件数量、manifest 元数据和文件指纹。",
            ),
            section(
                "安全提醒",
                "插件自带资源归插件所有，当前不由 WAPC 直接写入，避免破坏插件升级和同步机制。",
            ),
        ],
        "subagent" => vec![
            section(
                "用途",
                "Subagents 通过 frontmatter 描述名称、模型和工具权限。WAPC 只保存元数据与正文结构指纹。",
            ),
            section(
                "安全提醒",
                "正文可能包含工作流和内部策略，默认不进入数据库、导出报告或同步预设。",
            ),
        ],
        _ => vec![section(
            "安全边界",
            "未知资源类型只展示已识别元数据，不开放写入、同步或模板安装入口。",
        )],
    }
}

fn section(title: &str, body: &str) -> ResourceGuideSection {
    ResourceGuideSection {
        title: title.to_string(),
        body: body.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_center_links_mcp_guides_to_tool_and_kind_without_secret_values() {
        let guide = get_resource_guide(Some("claude"), Some("mcp"), Some("mcp:user:claude:github"));
        let text = serde_json::to_string(&guide).unwrap();

        assert_eq!(guide.id, "guide:claude:mcp");
        assert_eq!(guide.title, "Claude Code MCP 使用说明");
        assert!(
            guide
                .sections
                .iter()
                .any(|section| section.body.contains("Sync Engine"))
        );
        assert!(
            guide
                .unsupported_actions
                .iter()
                .any(|item| item.contains("enterprise"))
        );
        assert!(!text.contains("secret-token"));
    }

    #[test]
    fn guide_center_infers_kind_from_resource_id_when_kind_is_missing() {
        let guide = get_resource_guide(Some("codex"), None, Some("instruction:user:codex:agents"));

        assert_eq!(guide.id, "guide:codex:instruction");
        assert_eq!(guide.kind, "instruction");
        assert!(guide.summary.contains("指令文件"));
    }
}
