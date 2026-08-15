		const providerWizardCss = __PROVIDER_WIZARD_CSS__;
		if (typeof document !== "undefined" && document.querySelector('style[data-plugin-css="dsh-studio/provider-wizard"]') === null) {
			const tag = document.createElement("style");
			tag.dataset.plugin = "dsh-studio";
			tag.dataset.pluginCss = "dsh-studio/provider-wizard";
			tag.textContent = providerWizardCss;
			document.head.appendChild(tag);
		}
		function providerSetupLanguage() {
			const language = document.documentElement.lang || navigator.language || "en";
			return language.toLowerCase().startsWith("zh") ? "zh" : "en";
		}
		function localized(value, language) {
			return typeof value === "string" ? value : value[language];
		}
		function providerSetupText(copy, provider) {
			return copy.replace("{provider}", provider);
		}
		function providerRow(provider, rows) {
			return rows.find((row) => provider.aliases?.includes(row.entry.provider));
		}
		function customAvailable(provider, state) {
			if (provider.custom !== true && provider.preset === void 0) return false;
			return state.writable && state.namespaces.has("llm-pi-ai") && protocolChoices(state.namespaces.get("llm-pi-ai")).length > 0;
		}
		function ProviderCard({ provider, language, selected, disabled, onSelect }) {
			return (0, react_jsx_runtime.jsxs)("button", {
				type: "button",
				className: "provider-wizard__card",
				"aria-pressed": selected,
				disabled,
				onClick: onSelect,
				children: [(0, react_jsx_runtime.jsx)("span", {
					className: `provider-wizard__mark provider-wizard__mark--${provider.accent}`,
					"aria-hidden": true,
					children: provider.mark
				}), (0, react_jsx_runtime.jsxs)("span", {
					className: "provider-wizard__card-copy",
					children: [(0, react_jsx_runtime.jsx)("span", {
						className: "provider-wizard__card-name",
						children: provider.label
					}), (0, react_jsx_runtime.jsx)("span", {
						className: "provider-wizard__card-description",
						children: localized(provider.description, language)
					})]
				})]
			});
		}
		function WizardProgress({ step, copy, onSkip }) {
			return (0, react_jsx_runtime.jsxs)(react_jsx_runtime.Fragment, {
				children: [(0, react_jsx_runtime.jsxs)("div", {
					className: "provider-wizard__topline",
					children: [(0, react_jsx_runtime.jsx)("span", {
						className: "provider-wizard__step",
						children: step === 1 ? copy.stepOne : copy.stepTwo
					}), (0, react_jsx_runtime.jsx)("button", {
						type: "button",
						className: "provider-wizard__skip",
						onClick: onSkip,
						children: copy.skip
					})]
				}), (0, react_jsx_runtime.jsx)("div", {
					className: "provider-wizard__progress",
					"data-step": String(step),
					"aria-hidden": true,
					children: (0, react_jsx_runtime.jsx)("span", {})
				})]
			});
		}
		function ProviderConfiguration({ provider, state, controller, api, t, onBack }) {
			const row = providerRow(provider, state.rows);
			const finish = (changed) => {
				if (!changed) {
					onBack();
					return;
				}
				controller.load();
			};
			if (row !== void 0) {
				const namespace = state.namespaces.get(row.entry.settingsNs);
				if (namespace === void 0) return null;
				return (0, react_jsx_runtime.jsx)(ProviderEditor, {
					provider: row.entry.provider,
					displayName: row.entry.displayName,
					namespace,
					settingsPath: row.entry.settingsPath,
					api,
					t,
					readOnly: !state.writable,
					hideTitle: true,
					onClose: finish
				});
			}
			const namespace = state.namespaces.get("llm-pi-ai");
			if (namespace === void 0 || !customAvailable(provider, state)) return null;
			const preset = provider.preset ?? {};
			return (0, react_jsx_runtime.jsx)(CustomProviderCard, {
				taken: state.rows.map((candidate) => candidate.entry.provider),
				protocols: protocolChoices(namespace),
				revision: namespace.revision,
				api,
				t,
				readOnly: !state.writable,
				initialRoute: preset.route,
				initialDisplayName: preset.displayName,
				initialBaseURL: preset.baseURL,
				initialProtocol: preset.api,
				onClose: finish
			});
		}
		function ProviderSetupWizard(props) {
			const { complete, controller, setupController, useModels, useProviderSetup, api, t } = props;
			const state = useModels((snapshot) => snapshot);
			const setup = useProviderSetup((snapshot) => snapshot);
			const [selectedId, setSelectedId] = (0, react.useState)(void 0);
			const [step, setStep] = (0, react.useState)(1);
			const finishing = (0, react.useRef)(false);
			const language = providerSetupLanguage();
			const copy = PROVIDER_SETUP_COPY[language];
			const providers = PROVIDER_GROUPS.flatMap((group) => group.providers);
			const selected = providers.find((provider) => provider.id === selectedId);
			const finish = (0, react.useCallback)(async () => {
				if (finishing.current) return;
				finishing.current = true;
				await setupController.complete();
				complete();
			}, [complete, setupController]);
			(0, react.useEffect)(() => {
				if (state.status === "idle") controller.load();
				if (setup.status === "idle") setupController.load();
			}, [controller, setupController, setup.status, state.status]);
			const anyUsable = state.rows.some(providerUsable);
			(0, react.useEffect)(() => {
				if (setup.completed) {
					complete();
					return;
				}
				if (state.status === "error") {
					complete();
					return;
				}
				if (state.status === "ready" && anyUsable) finish();
			}, [anyUsable, complete, finish, setup.completed, state.status]);
			if (setup.status === "idle" || setup.status === "loading" || state.status === "idle" || state.status === "loading") return null;
			if (setup.completed || state.status === "error" || anyUsable) return null;
			const skip = () => { finish(); };
			const showingSelection = step === 1 || selected === void 0;
			const title = showingSelection ? copy.title : providerSetupText(copy.configure, selected.label);
			const available = selected !== void 0 && (providerRow(selected, state.rows) !== void 0 || customAvailable(selected, state));
			return (0, react_jsx_runtime.jsx)(OnboardingModal, {
				title,
				children: (0, react_jsx_runtime.jsxs)("div", {
					className: "provider-wizard",
					children: [(0, react_jsx_runtime.jsx)(WizardProgress, { step: showingSelection ? 1 : 2, copy, onSkip: skip }), (0, react_jsx_runtime.jsxs)("div", {
						className: "provider-wizard__selection",
						hidden: !showingSelection,
						children: [(0, react_jsx_runtime.jsx)("p", {
							className: "provider-wizard__description",
							children: copy.description
						}), PROVIDER_GROUPS.map((group) => (0, react_jsx_runtime.jsxs)("section", {
							className: "provider-wizard__group",
							children: [(0, react_jsx_runtime.jsx)("h3", {
								className: "provider-wizard__group-title",
								children: localized(group.label, language)
							}), (0, react_jsx_runtime.jsx)("div", {
								className: "provider-wizard__grid",
								children: group.providers.map((provider) => {
									const providerAvailable = providerRow(provider, state.rows) !== void 0 || customAvailable(provider, state);
									return (0, react_jsx_runtime.jsx)(ProviderCard, {
										provider,
										language,
										selected: selectedId === provider.id,
										disabled: !providerAvailable,
										onSelect: () => { setSelectedId(provider.id); }
									}, provider.id);
								})
							})]
						}, group.id)), (0, react_jsx_runtime.jsxs)("div", {
							className: "provider-wizard__footer",
							children: [(0, react_jsx_runtime.jsx)("span", {
								className: "provider-wizard__privacy",
								children: copy.privacy
							}), (0, react_jsx_runtime.jsx)("button", {
								type: "button",
								className: "provider-wizard__primary",
								disabled: selectedId === void 0,
								onClick: () => { setStep(2); },
								children: copy.continue
							})]
						})]
					}), selected !== void 0 && (0, react_jsx_runtime.jsxs)("div", {
						className: "provider-wizard__configuration",
						hidden: showingSelection,
						children: [(0, react_jsx_runtime.jsx)("button", {
							type: "button",
							className: "provider-wizard__back",
							onClick: () => { setStep(1); },
							children: `← ${copy.back}`
						}), (0, react_jsx_runtime.jsx)("p", {
							className: "provider-wizard__description",
							children: copy.configureDescription
						}), available ? (0, react_jsx_runtime.jsx)("div", {
							className: "provider-wizard__editor",
							children: (0, react_jsx_runtime.jsx)(ProviderConfiguration, {
								provider: selected,
								state,
								controller,
								api,
								t,
								onBack: () => { setStep(1); }
							}, selected.id)
						}) : (0, react_jsx_runtime.jsx)("p", {
							className: "provider-wizard__error",
							role: "alert",
							children: copy.unavailable
						})]
					})]
				})
			});
		}
