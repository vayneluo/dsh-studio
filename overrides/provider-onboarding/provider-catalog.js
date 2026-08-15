		const PROVIDER_GROUPS = [
			{
				id: "domestic",
				label: { zh: "国内服务", en: "Domestic services" },
				providers: [
					{
						id: "deepseek",
						label: "DeepSeek",
						description: { zh: "官方 API", en: "Official API" },
						aliases: ["deepseek-official", "deepseek"],
						mark: "DS",
						accent: "deepseek"
					},
					{
						id: "bailian",
						label: "阿里云百炼",
						description: { zh: "DashScope 兼容接口", en: "DashScope compatible API" },
						aliases: ["alibaba", "qwen", "bailian"],
						mark: "Ali",
						accent: "bailian",
						preset: {
							route: "bailian",
							displayName: "阿里云百炼",
							baseURL: "https://dashscope.aliyuncs.com/compatible-mode/v1",
							api: "openai-completions"
						}
					},
					{
						id: "ark",
						label: "火山方舟",
						description: { zh: "豆包与方舟模型", en: "Doubao and Ark models" },
						aliases: ["volcengine", "doubao", "volcengine-ark"],
						mark: "V",
						accent: "ark",
						preset: {
							route: "volcengine-ark",
							displayName: "火山方舟",
							baseURL: "https://ark.cn-beijing.volces.com/api/v3",
							api: "openai-completions"
						}
					},
					{
						id: "zhipu",
						label: "智谱 AI",
						description: { zh: "GLM 系列模型", en: "GLM model family" },
						aliases: ["zhipu", "zai"],
						mark: "Z",
						accent: "zhipu",
						preset: {
							route: "zhipu",
							displayName: "智谱 AI",
							baseURL: "https://open.bigmodel.cn/api/paas/v4",
							api: "openai-completions"
						}
					}
				]
			},
			{
				id: "international",
				label: { zh: "国际服务", en: "International services" },
				providers: [
					{ id: "openai", label: "OpenAI", description: { zh: "GPT 系列", en: "GPT models" }, aliases: ["openai"], mark: "AI", accent: "openai" },
					{ id: "anthropic", label: "Anthropic", description: { zh: "Claude 系列", en: "Claude models" }, aliases: ["anthropic"], mark: "A", accent: "anthropic" },
					{ id: "gemini", label: "Gemini", description: { zh: "Google AI", en: "Google AI" }, aliases: ["google", "gemini"], mark: "G", accent: "gemini" },
					{ id: "openrouter", label: "OpenRouter", description: { zh: "多模型网关", en: "Multi-model gateway" }, aliases: ["openrouter"], mark: "OR", accent: "openrouter" }
				]
			},
			{
				id: "local-custom",
				label: { zh: "本地与自定义", en: "Local and custom" },
				providers: [
					{ id: "ollama", label: "Ollama", description: { zh: "连接本地模型", en: "Connect local models" }, aliases: ["ollama"], mark: "O", accent: "ollama" },
					{ id: "compatible", label: "OpenAI Compatible", description: { zh: "自定义 Base URL 与模型", en: "Custom Base URL and models" }, custom: true, mark: "↗", accent: "compatible" }
				]
			}
		];
		const PROVIDER_SETUP_COPY = {
			zh: {
				title: "选择你的模型服务",
				description: "选择一个常用服务开始。稍后仍可在“设置 → 模型”中添加或切换。",
				stepOne: "第 1 步，共 2 步",
				stepTwo: "第 2 步，共 2 步",
				privacy: "API 密钥仅保存在本机凭据存储",
				skip: "跳过，稍后配置",
				continue: "继续",
				back: "返回选择",
				configure: "连接 {provider}",
				configureDescription: "保存后立即可用，不需要重启应用。",
				unavailable: "当前运行时没有可用于此服务的配置适配器。",
				persistenceFailure: "无法保存跳过状态，下次启动可能再次显示。"
			},
			en: {
				title: "Choose your model service",
				description: "Choose a common service to begin. You can add or switch providers later in Settings → Models.",
				stepOne: "Step 1 of 2",
				stepTwo: "Step 2 of 2",
				privacy: "API keys stay in the local credential store",
				skip: "Skip and configure later",
				continue: "Continue",
				back: "Back to providers",
				configure: "Connect {provider}",
				configureDescription: "Changes take effect immediately; no restart is required.",
				unavailable: "This runtime has no configurable adapter for this service.",
				persistenceFailure: "The skip preference could not be saved and may appear again next time."
			}
		};
