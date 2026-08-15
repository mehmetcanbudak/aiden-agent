use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

/// App-owned assets layered over gpui-component's icon bundle.
pub struct AppAssets;

const APP_ASSET_PATHS: &[&str] = &[
    "native-icons/blocks.svg",
    "native-icons/folder-git-2.svg",
    "native-icons/lock.svg",
    "native-icons/log-in.svg",
    "native-icons/network.svg",
    "native-icons/octagon-alert.svg",
    "native-icons/refresh-cw.svg",
    "native-icons/shield-question.svg",
    "native-icons/mcp/composio.svg",
    "native-icons/mcp/linear.svg",
    "native-icons/mcp/notion.svg",
    "native-icons/settings/chart-scatter.svg",
    "native-icons/settings/clock-3.svg",
    "native-icons/settings/file-up.svg",
    "native-icons/settings/globe.svg",
    "native-icons/settings/info.svg",
    "native-icons/settings/keyboard.svg",
    "native-icons/settings/list-filter.svg",
    "native-icons/settings/mic.svg",
    "native-icons/settings/mouse-pointer-2.svg",
    "native-icons/settings/palette.svg",
    "native-icons/settings/plug.svg",
    "native-icons/settings/server.svg",
    "native-icons/settings/sparkles.svg",
    "native-icons/settings/wand-sparkles.svg",
    "provider-logos/amazon-bedrock.svg",
    "provider-logos/ant-ling.svg",
    "provider-logos/anthropic.svg",
    "provider-logos/apple-foundation-models.svg",
    "provider-logos/azure-openai-responses.svg",
    "provider-logos/cerebras.svg",
    "provider-logos/claude.svg",
    "provider-logos/cloudflare-ai-gateway.svg",
    "provider-logos/cloudflare-workers-ai.svg",
    "provider-logos/deepseek.svg",
    "provider-logos/fireworks.svg",
    "provider-logos/github-copilot.svg",
    "provider-logos/google.svg",
    "provider-logos/google-vertex.svg",
    "provider-logos/grok.svg",
    "provider-logos/groq.svg",
    "provider-logos/huggingface.svg",
    "provider-logos/kimi-coding.svg",
    "provider-logos/lmstudio.svg",
    "provider-logos/mistral.svg",
    "provider-logos/minimax.svg",
    "provider-logos/minimax-cn.svg",
    "provider-logos/moonshotai.svg",
    "provider-logos/moonshotai-cn.svg",
    "provider-logos/nvidia.svg",
    "provider-logos/ollama.svg",
    "provider-logos/openai.svg",
    "provider-logos/openai-codex.svg",
    "provider-logos/openrouter.svg",
    "provider-logos/opencode.svg",
    "provider-logos/opencode-go.svg",
    "provider-logos/together.svg",
    "provider-logos/vercel-ai-gateway.svg",
    "provider-logos/xai.svg",
    "provider-logos/xiaomi.svg",
    "provider-logos/xiaomi-token-plan-ams.svg",
    "provider-logos/xiaomi-token-plan-cn.svg",
    "provider-logos/xiaomi-token-plan-sgp.svg",
    "provider-logos/zai.svg",
    "provider-logos/zai-coding-cn.svg",
];

fn app_asset(path: &str) -> Option<&'static [u8]> {
    Some(match path {
        "native-icons/blocks.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/blocks.svg")
        }
        "native-icons/folder-git-2.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/folder-git-2.svg")
        }
        "native-icons/lock.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/lock.svg")
        }
        "native-icons/log-in.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/log-in.svg")
        }
        "native-icons/network.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/network.svg")
        }
        "native-icons/octagon-alert.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/octagon-alert.svg")
        }
        "native-icons/refresh-cw.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/refresh-cw.svg")
        }
        "native-icons/shield-question.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/shield-question.svg")
        }
        "native-icons/mcp/composio.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/mcp/composio.svg")
        }
        "native-icons/mcp/linear.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/mcp/linear.svg")
        }
        "native-icons/mcp/notion.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/mcp/notion.svg")
        }
        "native-icons/settings/chart-scatter.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/chart-scatter.svg")
        }
        "native-icons/settings/clock-3.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/clock-3.svg")
        }
        "native-icons/settings/file-up.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/file-up.svg")
        }
        "native-icons/settings/globe.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/globe.svg")
        }
        "native-icons/settings/info.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/info.svg")
        }
        "native-icons/settings/keyboard.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/keyboard.svg")
        }
        "native-icons/settings/list-filter.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/list-filter.svg")
        }
        "native-icons/settings/mic.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/mic.svg")
        }
        "native-icons/settings/mouse-pointer-2.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/mouse-pointer-2.svg")
        }
        "native-icons/settings/palette.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/palette.svg")
        }
        "native-icons/settings/plug.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/plug.svg")
        }
        "native-icons/settings/server.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/server.svg")
        }
        "native-icons/settings/sparkles.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/sparkles.svg")
        }
        "native-icons/settings/wand-sparkles.svg" => {
            include_bytes!("../../../renderer/assets/native-icons/settings/wand-sparkles.svg")
        }
        "provider-logos/amazon-bedrock.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/amazon-bedrock.svg")
        }
        "provider-logos/ant-ling.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/ant-ling.svg")
        }
        "provider-logos/anthropic.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/anthropic.svg")
        }
        "provider-logos/apple-foundation-models.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/apple-foundation-models.svg")
        }
        "provider-logos/azure-openai-responses.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/azure-openai-responses.svg")
        }
        "provider-logos/cerebras.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/cerebras.svg")
        }
        "provider-logos/claude.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/claude.svg")
        }
        "provider-logos/cloudflare-ai-gateway.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/cloudflare-ai-gateway.svg")
        }
        "provider-logos/cloudflare-workers-ai.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/cloudflare-workers-ai.svg")
        }
        "provider-logos/deepseek.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/deepseek.svg")
        }
        "provider-logos/fireworks.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/fireworks.svg")
        }
        "provider-logos/github-copilot.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/github-copilot.svg")
        }
        "provider-logos/google.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/google.svg")
        }
        "provider-logos/google-vertex.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/google-vertex.svg")
        }
        "provider-logos/grok.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/grok.svg")
        }
        "provider-logos/groq.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/groq.svg")
        }
        "provider-logos/huggingface.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/huggingface.svg")
        }
        "provider-logos/kimi-coding.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/kimi-coding.svg")
        }
        "provider-logos/lmstudio.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/lmstudio.svg")
        }
        "provider-logos/mistral.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/mistral.svg")
        }
        "provider-logos/minimax.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/minimax.svg")
        }
        "provider-logos/minimax-cn.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/minimax-cn.svg")
        }
        "provider-logos/moonshotai.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/moonshotai.svg")
        }
        "provider-logos/moonshotai-cn.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/moonshotai-cn.svg")
        }
        "provider-logos/nvidia.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/nvidia.svg")
        }
        "provider-logos/ollama.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/ollama.svg")
        }
        "provider-logos/openai.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/openai.svg")
        }
        "provider-logos/openai-codex.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/openai-codex.svg")
        }
        "provider-logos/openrouter.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/openrouter.svg")
        }
        "provider-logos/opencode.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/opencode.svg")
        }
        "provider-logos/opencode-go.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/opencode-go.svg")
        }
        "provider-logos/together.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/together.svg")
        }
        "provider-logos/vercel-ai-gateway.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/vercel-ai-gateway.svg")
        }
        "provider-logos/xai.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/xai.svg")
        }
        "provider-logos/xiaomi.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/xiaomi.svg")
        }
        "provider-logos/xiaomi-token-plan-ams.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/xiaomi-token-plan-ams.svg")
        }
        "provider-logos/xiaomi-token-plan-cn.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/xiaomi-token-plan-cn.svg")
        }
        "provider-logos/xiaomi-token-plan-sgp.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/xiaomi-token-plan-sgp.svg")
        }
        "provider-logos/zai.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/zai.svg")
        }
        "provider-logos/zai-coding-cn.svg" => {
            include_bytes!("../../../renderer/assets/provider-logos/zai-coding-cn.svg")
        }
        _ => return None,
    })
}

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(asset) = app_asset(path) {
            return Ok(Some(Cow::Borrowed(asset)));
        }
        gpui_component_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut assets = gpui_component_assets::Assets.list(path)?;
        assets.extend(
            APP_ASSET_PATHS
                .iter()
                .filter(|candidate| candidate.starts_with(path))
                .map(|candidate| SharedString::from(*candidate)),
        );
        Ok(assets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_assets_are_embedded_alongside_component_icons() {
        assert!(AppAssets
            .load("provider-logos/anthropic.svg")
            .unwrap()
            .is_some());
        assert!(AppAssets
            .load("native-icons/network.svg")
            .unwrap()
            .is_some());
        assert!(AppAssets
            .load("native-icons/settings/chart-scatter.svg")
            .unwrap()
            .is_some());
        assert!(AppAssets
            .load("native-icons/settings/mouse-pointer-2.svg")
            .unwrap()
            .is_some());
        assert!(AppAssets.load("icons/check.svg").unwrap().is_some());
    }
}
