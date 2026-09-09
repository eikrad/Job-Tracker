//! Data-driven provider registry (spec §8.1).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProvider {
    ScalewayDeepseek,
    Mistral,
    Gemini,
}

impl LlmProvider {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim() {
            "scaleway_deepseek" | "scaleway" => Ok(Self::ScalewayDeepseek),
            "mistral" => Ok(Self::Mistral),
            "gemini" => Ok(Self::Gemini),
            other => Err(format!("Unknown LLM provider: {other}")),
        }
    }

    #[allow(dead_code)]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ScalewayDeepseek => "scaleway_deepseek",
            Self::Mistral => "mistral",
            Self::Gemini => "gemini",
        }
    }

    pub fn secret_provider(self) -> &'static str {
        match self {
            Self::ScalewayDeepseek => "scaleway",
            Self::Mistral => "mistral",
            Self::Gemini => "gemini",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AuthStyle {
    Bearer,
    Header(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub enum JsonMode {
    /// OpenAI-style `response_format: { type: "json_schema", ... }` (preferred for Scaleway).
    Schema,
    /// OpenAI/Mistral `response_format: { type: "json_object" }`.
    JsonObject,
    /// Gemini `generationConfig.responseMimeType = application/json`.
    ResponseMimeType,
}

#[derive(Debug, Clone)]
pub struct ProviderSpec {
    #[allow(dead_code)]
    pub id: &'static str,
    pub base_url: &'static str,
    pub model_id: &'static str,
    pub auth: AuthStyle,
    pub json_mode: JsonMode,
}

pub fn provider_spec(provider: LlmProvider) -> ProviderSpec {
    match provider {
        LlmProvider::ScalewayDeepseek => ProviderSpec {
            id: "scaleway_deepseek",
            base_url: "https://api.scaleway.ai/v1",
            // Default from Scaleway catalogue (2026-09); overridable later in Settings.
            model_id: "deepseek-v4-flash-0731",
            auth: AuthStyle::Bearer,
            json_mode: JsonMode::Schema,
        },
        LlmProvider::Mistral => ProviderSpec {
            id: "mistral",
            base_url: "https://api.mistral.ai/v1",
            model_id: "mistral-small-latest",
            auth: AuthStyle::Bearer,
            json_mode: JsonMode::JsonObject,
        },
        LlmProvider::Gemini => ProviderSpec {
            id: "gemini",
            base_url: "https://generativelanguage.googleapis.com/v1beta",
            model_id: "gemini-2.0-flash",
            auth: AuthStyle::Header("x-goog-api-key"),
            json_mode: JsonMode::ResponseMimeType,
        },
    }
}
