//! Shared usage model for normalized AI coding tool token records.
//! @author codex

use std::ops::Add;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub reasoning: u64,
    pub tool: u64,
}

impl TokenUsage {
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_read + self.cache_write + self.reasoning + self.tool
    }
}

impl Add for TokenUsage {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            input: self.input + rhs.input,
            output: self.output + rhs.output,
            cache_read: self.cache_read + rhs.cache_read,
            cache_write: self.cache_write + rhs.cache_write,
            reasoning: self.reasoning + rhs.reasoning,
            tool: self.tool + rhs.tool,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SourcePrecision {
    Exact,
    Computed,
    Estimated,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ToolKind {
    Claude,
    Codex,
    Gemini,
    OpenCode,
}

impl ToolKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::OpenCode => "opencode",
        }
    }
}

impl FromStr for ToolKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            "gemini" => Ok(Self::Gemini),
            "opencode" => Ok(Self::OpenCode),
            other => Err(format!("unknown tool: {other}")),
        }
    }
}

impl SourcePrecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Computed => "computed",
            Self::Estimated => "estimated",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub tool: ToolKind,
    pub source_path: String,
    pub session_id: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub project_path: Option<String>,
    pub model: Option<String>,
    pub usage: TokenUsage,
    pub cost_usd: Option<f64>,
    pub precision: SourcePrecision,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_usage_total_includes_all_billable_buckets() {
        let usage = TokenUsage {
            input: 10,
            output: 20,
            cache_read: 30,
            cache_write: 40,
            reasoning: 50,
            tool: 60,
        };

        assert_eq!(usage.total(), 210);
    }

    #[test]
    fn token_usage_adds_two_records_bucket_by_bucket() {
        let left = TokenUsage {
            input: 1,
            output: 2,
            cache_read: 3,
            cache_write: 4,
            reasoning: 5,
            tool: 6,
        };
        let right = TokenUsage {
            input: 10,
            output: 20,
            cache_read: 30,
            cache_write: 40,
            reasoning: 50,
            tool: 60,
        };

        assert_eq!(
            left + right,
            TokenUsage {
                input: 11,
                output: 22,
                cache_read: 33,
                cache_write: 44,
                reasoning: 55,
                tool: 66,
            }
        );
    }
}
