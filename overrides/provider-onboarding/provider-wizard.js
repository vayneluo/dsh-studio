		const PROVIDER_SETUP_VERSION = "1";
		const PROVIDER_SETUP_FIELD = "providerSetupVersion";
		const PROVIDER_SETUP_NAMESPACE = "ui-onboarding";
		class ProviderSetupStore {
			store = (0, _deepseek_ai_dsh_client_runtime_client.createSnapshotStore)({
				status: "idle",
				completed: false,
				error: null
			});
			generation = 0;
			constructor(api, persistence = "host") {
				this.api = api;
				this.persistence = persistence;
			}
			async load() {
				const generation = ++this.generation;
				if (this.persistence === "memory") {
					this.store.update((state) => {
						state.status = "ready";
						state.error = null;
					});
					return;
				}
				this.store.update((state) => {
					state.status = "loading";
					state.error = null;
				});
				try {
					const response = await this.api.settings.describe({});
					if (!response.result.ok) throw new Error(response.result.error.message);
					const view = response.result.value.namespaces.find((item) => item.ns === PROVIDER_SETUP_NAMESPACE);
					const value = typeof view?.value === "object" && view.value !== null ? view.value[PROVIDER_SETUP_FIELD] : void 0;
					if (generation !== this.generation) return;
					this.store.update((state) => {
						state.status = "ready";
						state.completed = value === PROVIDER_SETUP_VERSION;
						state.error = null;
					});
				} catch (error) {
					if (generation !== this.generation) return;
					this.store.update((state) => {
						state.status = "error";
						state.completed = false;
						state.error = error instanceof Error ? error.message : String(error);
					});
				}
			}
			async complete() {
				if (this.persistence === "memory") {
					this.store.update((state) => {
						state.status = "ready";
						state.completed = true;
						state.error = null;
					});
					return true;
				}
				try {
					const response = await this.api.settings.mutate({
						ns: PROVIDER_SETUP_NAMESPACE,
						ops: [{ op: "set", path: [PROVIDER_SETUP_FIELD], value: PROVIDER_SETUP_VERSION }]
					});
					if (!response.result.ok) throw new Error(response.result.error.message);
					this.store.update((state) => {
						state.status = "ready";
						state.completed = true;
						state.error = null;
					});
					return true;
				} catch (error) {
					this.store.update((state) => {
						state.status = "error";
						state.error = error instanceof Error ? error.message : String(error);
					});
					return false;
				}
			}
		}
		function refreshProviderSetupIfLoaded(controller) {
			if (controller.store.getSnapshot().status === "idle") return;
			controller.load();
		}
